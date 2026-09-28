//! Gestión de una conexión cliente dentro del servidor.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use crate::controlador::protocolo::{Mensaje, Operation, Resultado, Status};
use crate::controlador::traductor;
use crate::controlador::usuario::Usuario;

/// Diccionario de usuarios compartido por los hilos del servidor.
pub type Usuarios = Arc<Mutex<HashMap<String, Usuario>>>;

/// Lee, valida y atiende una conexión TCP autenticada.
pub struct Manejador {
    reader: BufReader<TcpStream>,
    usuarios: Usuarios,
    nombre: Option<String>,
}

impl Manejador {
    /// Crea un manejador asociado al stream y al registro global de usuarios.
    pub fn nuevo(stream: TcpStream, usuarios: Usuarios) -> Self {
        Self {
            reader: BufReader::new(stream),
            usuarios,
            nombre: None,
        }
    }

    /// Serializa y envía un mensaje a este cliente.
    pub fn enviar(&mut self, mensaje: &Mensaje) -> Result<(), Error> {
        let mut json = traductor::serializa(mensaje)?;
        json.push('\n');

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

                let mut usuarios = self
                    .usuarios
                    .lock()
                    .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;

                if let Entry::Occupied(_) = usuarios.entry(username.clone()) {
                    drop(usuarios);

                    let respuesta = Mensaje::Response {
                        operation: Operation::Identify,
                        result: Resultado::UserAlreadyExist,
                        extra: Some(username),
                    };

                    self.enviar(&respuesta)?;

                    return Ok(false);
                }

                usuarios.insert(
                    username.clone(),
                    Usuario {
                        nombre: username.clone(),
                        stream: self.reader.get_ref().try_clone()?,
                        status: Status::Active,
                    },
                );
                drop(usuarios);
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
        let resultado = loop {
            let mensaje = match self.leer() {
                Ok(mensaje) => mensaje,
                Err(error) if error.kind() == ErrorKind::UnexpectedEof => break Ok(()),
                Err(error) => break Err(error),
            };

            if let Mensaje::PublicText { text } = mensaje {
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

                let mut usuarios = self
                    .usuarios
                    .lock()
                    .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;
                if let Some(usuario) = usuarios.get_mut(username) {
                    usuario.status = status;
                }
                drop(usuarios);

                self.difundir(&Mensaje::NewStatus {
                    username: username.clone(),
                    status,
                })?;
            } else if let Mensaje::PrivateText { username, text } = mensaje {
                if text.trim().is_empty() {
                    continue;
                }

                let destinatario = {
                    let usuarios = self
                        .usuarios
                        .lock()
                        .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;

                    usuarios
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

            let mut usuarios = self
                .usuarios
                .lock()
                .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;
            usuarios.remove(&nombre);
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
            let usuarios = self
                .usuarios
                .lock()
                .map_err(|_| Error::other("No se pudo acceder a los usuarios"))?;

            let mut destinatarios = Vec::new();

            for usuario in usuarios
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
