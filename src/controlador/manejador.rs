//! Gestión de una conexión cliente dentro del servidor.

use std::collections::hash_map::Entry;
use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::net::TcpStream;

use crate::controlador::cuarto::Cuarto;
use crate::controlador::estado::EstadoCompartido;
use crate::controlador::protocolo::{Mensaje, Operation, Resultado, Status};
use crate::controlador::traductor;
use crate::controlador::usuario::Usuario;

/// Lee, valida y atiende una conexión TCP autenticada.
pub struct Manejador {
    reader: BufReader<TcpStream>,
    estado: EstadoCompartido,
    nombre: Option<String>,
}

impl Manejador {
    /// Crea un manejador asociado al stream y al registro global de usuarios.
    pub fn nuevo(stream: TcpStream, estado: EstadoCompartido) -> Self {
        Self {
            reader: BufReader::new(stream),
            estado,
            nombre: None,
        }
    }

    /// Serializa y envía un mensaje a este cliente.
    pub fn enviar(&mut self, mensaje: &Mensaje) -> Result<(), Error> {
        let mut json = traductor::serializa(mensaje)?;
        json.push('\n');
        println!("{}", json.trim_end());

        self.reader.get_mut().write_all(json.as_bytes())?;

        Ok(())
    }

    /// Lee mensajes no vacíos, registra su JSON en el servidor y lo deserializa.
    pub fn leer(&mut self) -> Result<Mensaje, Error> {
        loop {
            let mut mensaje = String::new();

            if self.reader.read_line(&mut mensaje)? == 0 {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "El cliente se desconectó",
                ));
            }

            if mensaje.trim().is_empty() {
                continue;
            }

            println!("{}", mensaje.trim_end());
            return Ok(traductor::deserializa(&mensaje)?);
        }
    }

    /// Comprueba las reglas actuales para un nombre de usuario.
    fn nombre_valido(username: &str) -> bool {
        !username.is_empty() && username.chars().count() <= 8
    }

    /// Procesa el primer mensaje e incorpora al cliente si su nombre es válido y único.
    pub fn verificar(&mut self) -> Result<bool, Error> {
        let mensaje = self.leer()?;

        match mensaje {
            Mensaje::Identify { username } => {
                if !Self::nombre_valido(&username) {
                    let respuesta = Mensaje::Response {
                        operation: Operation::Identify,
                        result: Resultado::NotIdentified,
                        extra: None,
                    };

                    self.enviar(&respuesta)?;

                    return Ok(false);
                }

                let mut estado = self
                    .estado
                    .lock()
                    .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;

                if let Entry::Occupied(_) = estado.usuarios.entry(username.clone()) {
                    drop(estado);

                    let respuesta = Mensaje::Response {
                        operation: Operation::Identify,
                        result: Resultado::UserAlreadyExist,
                        extra: Some(username),
                    };

                    self.enviar(&respuesta)?;

                    return Ok(false);
                }

                estado.usuarios.insert(
                    username.clone(),
                    Usuario {
                        nombre: username.clone(),
                        stream: self.reader.get_ref().try_clone()?,
                        status: Status::Active,
                    },
                );
                drop(estado);
                self.nombre = Some(username.clone());

                self.difundir(&Mensaje::NewUser {
                    username: username.clone(),
                })?;

                let respuesta = Mensaje::Response {
                    operation: Operation::Identify,
                    result: Resultado::Success,
                    extra: Some(username),
                };

                self.enviar(&respuesta)?;

                Ok(true)
            }

            _ => {
                let respuesta = Mensaje::Response {
                    operation: Operation::Invalid,
                    result: Resultado::NotIdentified,
                    extra: None,
                };

                self.enviar(&respuesta)?;

                Ok(false)
            }
        }
    }

    /// Atiende mensajes hasta la desconexión y libera el nombre del usuario.
    pub fn atender(&mut self) -> Result<(), Error> {
        let resultado =
            loop {
                let mensaje = match self.leer() {
                    Ok(mensaje) => mensaje,
                    Err(error) if error.kind() == ErrorKind::UnexpectedEof => break Ok(()),
                    Err(error) => break Err(error),
                };

                if let Mensaje::Disconnect = mensaje {
                    break Ok(());
                } else if let Mensaje::Users = mensaje {
                    let users = {
                        let estado = self
                            .estado
                            .lock()
                            .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;

                        estado
                            .usuarios
                            .values()
                            .map(|usuario| (usuario.nombre.clone(), usuario.status))
                            .collect()
                    };

                    self.enviar(&Mensaje::UserList { users })?;
                } else if let Mensaje::RoomUsers { roomname } = mensaje {
                    let username = self
                        .nombre
                        .as_ref()
                        .ok_or_else(|| Error::other("El cliente no está identificado"))?
                        .clone();

                    let users = {
                        let estado = self.estado.lock().map_err(|_| {
                            Error::other("No se pudo acceder al estado del servidor")
                        })?;

                        let Some(cuarto) = estado.cuartos.get(&roomname) else {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::RoomUsers,
                                result: Resultado::NoSuchRoom,
                                extra: Some(roomname),
                            })?;
                            continue;
                        };

                        if !cuarto.miembros.contains(&username) {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::RoomUsers,
                                result: Resultado::NotJoined,
                                extra: Some(roomname),
                            })?;
                            continue;
                        }

                        cuarto
                            .miembros
                            .iter()
                            .filter_map(|miembro| {
                                estado
                                    .usuarios
                                    .get(miembro)
                                    .map(|usuario| (miembro.clone(), usuario.status))
                            })
                            .collect()
                    };

                    self.enviar(&Mensaje::RoomUserList { roomname, users })?;
                } else if let Mensaje::RoomText { roomname, text } = mensaje {
                    if text.trim().is_empty() {
                        continue;
                    }

                    let username = self
                        .nombre
                        .as_ref()
                        .ok_or_else(|| Error::other("El cliente no está identificado"))?
                        .clone();

                    let destinatarios = {
                        let estado = self.estado.lock().map_err(|_| {
                            Error::other("No se pudo acceder al estado del servidor")
                        })?;

                        let Some(cuarto) = estado.cuartos.get(&roomname) else {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::RoomText,
                                result: Resultado::NoSuchRoom,
                                extra: Some(roomname),
                            })?;
                            continue;
                        };

                        if !cuarto.miembros.contains(&username) {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::RoomText,
                                result: Resultado::NotJoined,
                                extra: Some(roomname),
                            })?;
                            continue;
                        }

                        cuarto
                            .miembros
                            .iter()
                            .filter(|miembro| miembro.as_str() != username)
                            .filter_map(|miembro| {
                                estado.usuarios.get(miembro).and_then(|usuario| {
                                    usuario
                                        .stream
                                        .try_clone()
                                        .ok()
                                        .map(|stream| (miembro.clone(), stream))
                                })
                            })
                            .collect::<Vec<_>>()
                    };

                    let mensaje = Mensaje::RoomTextFrom {
                        roomname,
                        username,
                        text,
                    };
                    let json = traductor::serializa(&mensaje)? + "\n";
                    println!("{}", json.trim_end());
                    for (destinatario, mut stream) in destinatarios {
                        if let Err(error) = stream.write_all(json.as_bytes()) {
                            eprintln!(
                                "No se pudo enviar el mensaje del cuarto a {}: {}",
                                destinatario, error
                            );
                        }
                    }
                } else if let Mensaje::LeaveRoom { roomname } = mensaje {
                    let username = self
                        .nombre
                        .as_ref()
                        .ok_or_else(|| Error::other("El cliente no está identificado"))?
                        .clone();

                    let destinatarios = {
                        let mut estado = self.estado.lock().map_err(|_| {
                            Error::other("No se pudo acceder al estado del servidor")
                        })?;

                        let Some(cuarto) = estado.cuartos.get_mut(&roomname) else {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::LeaveRoom,
                                result: Resultado::NoSuchRoom,
                                extra: Some(roomname),
                            })?;
                            continue;
                        };

                        if !cuarto.miembros.remove(&username) {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::LeaveRoom,
                                result: Resultado::NotJoined,
                                extra: Some(roomname),
                            })?;
                            continue;
                        }

                        let miembros = cuarto.miembros.iter().cloned().collect::<Vec<_>>();
                        if cuarto.esta_vacio() {
                            estado.cuartos.remove(&roomname);
                            Vec::new()
                        } else {
                            miembros
                                .iter()
                                .filter_map(|miembro| {
                                    estado
                                        .usuarios
                                        .get(miembro)
                                        .and_then(|usuario| usuario.stream.try_clone().ok())
                                })
                                .collect::<Vec<_>>()
                        }
                    };

                    let mensaje = Mensaje::LeftRoom { roomname, username };
                    let json = traductor::serializa(&mensaje)? + "\n";
                    println!("{}", json.trim_end());
                    for mut stream in destinatarios {
                        if let Err(error) = stream.write_all(json.as_bytes()) {
                            eprintln!("No se pudo notificar la salida del cuarto: {}", error);
                        }
                    }
                } else if let Mensaje::NewRoom { roomname } = mensaje {
                    let username = self
                        .nombre
                        .as_ref()
                        .ok_or_else(|| Error::other("El cliente no está identificado"))?;

                    if !Cuarto::nombre_valido(&roomname) {
                        self.enviar(&Mensaje::Response {
                            operation: Operation::NewRoom,
                            result: Resultado::Invalid,
                            extra: Some(roomname),
                        })?;
                        continue;
                    }

                    let mut estado = self
                        .estado
                        .lock()
                        .map_err(|_| Error::other("No se pudo acceder al estado del servidor"))?;

                    if estado.cuartos.contains_key(&roomname) {
                        drop(estado);
                        self.enviar(&Mensaje::Response {
                            operation: Operation::NewRoom,
                            result: Resultado::RoomAlreadyExists,
                            extra: Some(roomname),
                        })?;
                        continue;
                    }

                    estado.cuartos.insert(
                        roomname.clone(),
                        Cuarto::nuevo(roomname.clone(), username.clone()),
                    );
                    drop(estado);

                    self.enviar(&Mensaje::Response {
                        operation: Operation::NewRoom,
                        result: Resultado::Success,
                        extra: Some(roomname),
                    })?;
                } else if let Mensaje::Invite {
                    roomname,
                    usernames,
                } = mensaje
                {
                    let username = self
                        .nombre
                        .as_ref()
                        .ok_or_else(|| Error::other("El cliente no está identificado"))?
                        .clone();

                    let invitaciones = {
                        let mut estado = self.estado.lock().map_err(|_| {
                            Error::other("No se pudo acceder al estado del servidor")
                        })?;

                        let Some(cuarto) = estado.cuartos.get(&roomname) else {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::Invite,
                                result: Resultado::NoSuchRoom,
                                extra: Some(roomname),
                            })?;
                            continue;
                        };

                        if !cuarto.miembros.contains(&username) {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::Invite,
                                result: Resultado::Invalid,
                                extra: Some(roomname),
                            })?;
                            continue;
                        }

                        if let Some(no_existente) = usernames
                            .iter()
                            .find(|invitado| !estado.usuarios.contains_key(*invitado))
                        {
                            let no_existente = no_existente.clone();
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::Invite,
                                result: Resultado::NoSuchUser,
                                extra: Some(no_existente),
                            })?;
                            continue;
                        }

                        let mut invitaciones = Vec::new();
                        for invitado in usernames {
                            let puede_invitar = estado
                                .cuartos
                                .get(&roomname)
                                .map(|cuarto| {
                                    !cuarto.miembros.contains(&invitado)
                                        && !cuarto.invitados.contains(&invitado)
                                })
                                .unwrap_or(false);

                            if !puede_invitar {
                                continue;
                            }

                            if let Some(usuario) = estado.usuarios.get(&invitado) {
                                let stream = usuario.stream.try_clone()?;
                                estado
                                    .cuartos
                                    .get_mut(&roomname)
                                    .ok_or_else(|| Error::other("El cuarto dejó de existir"))?
                                    .invitados
                                    .insert(invitado.clone());
                                invitaciones.push((invitado, stream));
                            }
                        }

                        invitaciones
                    };

                    for (destinatario, mut stream) in invitaciones {
                        let mensaje = Mensaje::Invitation {
                            username: username.clone(),
                            roomname: roomname.clone(),
                        };
                        let json = traductor::serializa(&mensaje)? + "\n";
                        println!("{}", json.trim_end());
                        if let Err(error) = stream.write_all(json.as_bytes()) {
                            eprintln!(
                                "No se pudo enviar la invitación a {}: {}",
                                destinatario, error
                            );
                        }
                    }
                } else if let Mensaje::JoinRoom { roomname } = mensaje {
                    let username = self
                        .nombre
                        .as_ref()
                        .ok_or_else(|| Error::other("El cliente no está identificado"))?
                        .clone();

                    let destinatarios = {
                        let mut estado = self.estado.lock().map_err(|_| {
                            Error::other("No se pudo acceder al estado del servidor")
                        })?;

                        let Some(cuarto) = estado.cuartos.get_mut(&roomname) else {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::JoinRoom,
                                result: Resultado::NoSuchRoom,
                                extra: Some(roomname),
                            })?;
                            continue;
                        };

                        if !cuarto.invitados.remove(&username) {
                            drop(estado);
                            self.enviar(&Mensaje::Response {
                                operation: Operation::JoinRoom,
                                result: Resultado::NotInvited,
                                extra: Some(roomname),
                            })?;
                            continue;
                        }

                        cuarto.miembros.insert(username.clone());
                        let miembros = cuarto
                            .miembros
                            .iter()
                            .filter(|miembro| miembro.as_str() != username)
                            .cloned()
                            .collect::<Vec<_>>();

                        miembros
                            .iter()
                            .filter_map(|miembro| {
                                estado
                                    .usuarios
                                    .get(miembro)
                                    .and_then(|usuario| usuario.stream.try_clone().ok())
                            })
                            .collect::<Vec<_>>()
                    };

                    self.enviar(&Mensaje::Response {
                        operation: Operation::JoinRoom,
                        result: Resultado::Success,
                        extra: Some(roomname.clone()),
                    })?;

                    let mensaje = Mensaje::JoinedRoom { roomname, username };
                    let json = traductor::serializa(&mensaje)? + "\n";
                    println!("{}", json.trim_end());
                    for mut stream in destinatarios {
                        if let Err(error) = stream.write_all(json.as_bytes()) {
                            eprintln!("No se pudo notificar la unión al cuarto: {}", error);
                        }
                    }
                } else if let Mensaje::PublicText { text } = mensaje {
                    if text.trim().is_empty() {
                        continue;
                    }

                    let username = self
                        .nombre
                        .as_ref()
                        .ok_or_else(|| Error::other("El cliente no está identificado"))?;
                    let mensaje = Mensaje::PublicTextFrom {
                        username: username.clone(),
                        text,
                    };

                    self.difundir(&mensaje)?;
                } else if let Mensaje::Status { status } = mensaje {
                    let username = self
                        .nombre
                        .as_ref()
                        .ok_or_else(|| Error::other("El cliente no está identificado"))?;

                    let mut estado = self
                        .estado
                        .lock()
                        .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;
                    if let Some(usuario) = estado.usuarios.get_mut(username) {
                        usuario.status = status;
                    }
                    drop(estado);

                    self.difundir(&Mensaje::NewStatus {
                        username: username.clone(),
                        status,
                    })?;
                } else if let Mensaje::PrivateText { username, text } = mensaje {
                    if text.trim().is_empty() {
                        continue;
                    }

                    let destinatario = {
                        let estado = self
                            .estado
                            .lock()
                            .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;

                        estado
                            .usuarios
                            .get(&username)
                            .map(|usuario| (usuario.nombre.clone(), usuario.stream.try_clone()))
                    };

                    let Some((nombre, stream)) = destinatario else {
                        self.enviar(&Mensaje::Response {
                            operation: Operation::Text,
                            result: Resultado::NoSuchUser,
                            extra: Some(username),
                        })?;
                        continue;
                    };

                    match stream {
                        Ok(mut stream) => {
                            let mensaje = Mensaje::PrivateTextFrom {
                                username: self
                                    .nombre
                                    .as_ref()
                                    .ok_or_else(|| Error::other("El cliente no está identificado"))?
                                    .clone(),
                                text,
                            };
                            let json = traductor::serializa(&mensaje)? + "\n";
                            println!("{}", json.trim_end());
                            if let Err(error) = stream.write_all(json.as_bytes()) {
                                eprintln!("No se pudo enviar un mensaje a {}: {}", nombre, error);
                            }
                        }
                        Err(error) => {
                            eprintln!(
                                "No se pudo preparar el envío privado para {}: {}",
                                nombre, error
                            );
                        }
                    }
                }
            };

        if let Some(nombre) = self.nombre.take() {
            let notificacion = Mensaje::Disconnected {
                username: nombre.clone(),
            };
            if let Err(error) = self.difundir(&notificacion) {
                eprintln!("No se pudo notificar la desconexión: {}", error);
            }

            let mut estado = self
                .estado
                .lock()
                .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;
            estado.usuarios.remove(&nombre);
            estado.cuartos.retain(|_, cuarto| {
                cuarto.miembros.remove(&nombre);
                cuarto.invitados.remove(&nombre);
                !cuarto.esta_vacio()
            });
        }

        resultado
    }

    /// Envía un mensaje a todos los usuarios excepto a la conexión actual.
    ///
    /// El registro se bloquea solo para clonar los streams; las escrituras se
    /// realizan después de liberar el `Mutex` para no detener a otros hilos.
    fn difundir(&self, mensaje: &Mensaje) -> Result<(), Error> {
        let json = traductor::serializa(mensaje)? + "\n";
        println!("{}", json.trim_end());

        let destinatarios = {
            let estado = self
                .estado
                .lock()
                .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;

            let mut destinatarios = Vec::new();

            for usuario in estado
                .usuarios
                .values()
                .filter(|usuario| Some(usuario.nombre.as_str()) != self.nombre.as_deref())
            {
                match usuario.stream.try_clone() {
                    Ok(stream) => destinatarios.push((usuario.nombre.clone(), stream)),
                    Err(error) => eprintln!(
                        "No se pudo preparar el envío para {}: {}",
                        usuario.nombre, error
                    ),
                }
            }

            destinatarios
        };

        for (nombre, mut stream) in destinatarios {
            if let Err(error) = stream.write_all(json.as_bytes()) {
                eprintln!("No se pudo enviar un mensaje a {}: {}", nombre, error);
            }
        }

        Ok(())
    }
}
