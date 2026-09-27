use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use crate::controlador::protocolo::{Mensaje, Operation, Resultado};
use crate::controlador::traductor;
use crate::controlador::usuario::Usuario;

pub type Usuarios = Arc<Mutex<HashMap<String, Usuario>>>;

pub struct Manejador {
    reader: BufReader<TcpStream>,
    usuarios: Usuarios,
    nombre: Option<String>,
}

impl Manejador {
    pub fn nuevo(stream: TcpStream, usuarios: Usuarios) -> Self {
        Self {
            reader: BufReader::new(stream),
            usuarios,
            nombre: None,
        }
    }

    pub fn enviar(&mut self, mensaje: &Mensaje) -> Result<(), Error> {
        let mut json = traductor::serializa(mensaje)?;
        json.push('\n');

        self.reader.get_mut().write_all(json.as_bytes())?;

        Ok(())
    }

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

    fn nombre_valido(username: &str) -> bool {
        !username.is_empty() && username.chars().count() <= 8
    }

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
