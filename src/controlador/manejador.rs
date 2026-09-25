use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{BufRead, BufReader, Error, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use crate::controlador::protocolo::{Mensaje, Operation, Resultado};
use crate::controlador::traductor;
use crate::controlador::usuario::Usuario;

pub type Usuarios = Arc<Mutex<HashMap<String, Usuario>>>;

pub struct Manejador {
    reader: BufReader<TcpStream>,
    usuarios: Usuarios,
}

impl Manejador {
    pub fn nuevo(stream: TcpStream, usuarios: Usuarios) -> Self {
        Self {
            reader: BufReader::new(stream),
            usuarios,
        }
    }

    pub fn enviar(&mut self, mensaje: &Mensaje) -> Result<(), Error> {
        let mut json = traductor::serializa(mensaje)?;
        json.push('\n');

        self.reader.get_mut().write_all(json.as_bytes())?;

        Ok(())
    }

    pub fn leer(&mut self) -> Result<Mensaje, Error> {
        let mut mensaje = String::new();

        self.reader.read_line(&mut mensaje)?;

        println!("{}", mensaje.trim_end());

        let mensaje = traductor::deserializa(&mensaje)?;

        Ok(mensaje)
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
                    },
                );
                drop(usuarios);

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

    pub fn atender() {
        println!("Futura función que atiende al cliente");
    }
}
