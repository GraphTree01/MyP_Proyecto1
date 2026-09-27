use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::thread;

use crate::controlador::protocolo::Mensaje;
use crate::controlador::traductor;

pub struct Cliente {
    servidor: SocketAddrV4,
    nombre: Option<String>,
    stream: Option<TcpStream>,
}

impl Cliente {
    pub fn nuevo(ip: Ipv4Addr, puerto: u16) -> Self {
        Self {
            servidor: SocketAddrV4::new(ip, puerto),
            nombre: None,
            stream: None,
        }
    }

    pub fn conectar(&mut self) -> Result<(), Error> {
        let stream = TcpStream::connect(self.servidor)?;
        self.stream = Some(stream);

        Ok(())
    }

    pub fn identificar(&mut self, username: String) -> Result<(), Error> {
        let mensaje = Mensaje::Identify {
            username: username.clone(),
        };

        self.enviar(&mensaje)?;

        self.nombre = Some(username);

        Ok(())
    }

    pub fn enviar(&mut self, mensaje: &Mensaje) -> Result<(), Error> {
        let mut json = traductor::serializa(mensaje)?;
        json.push('\n');

        if let Some(stream) = self.stream.as_mut() {
            stream.write_all(json.as_bytes())?;
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::NotConnected,
                "El cliente no está conectado",
            ))
        }
    }

    pub fn procesar_comando(&mut self, comando: &str) -> Result<bool, Error> {
        let comando = comando.trim();
        if comando.is_empty() {
            return Ok(true);
        }

        let Some((nombre, text)) = comando.split_once(' ') else {
            return Ok(false);
        };

        if nombre != r"\publicText" || text.trim().is_empty() {
            return Ok(false);
        }

        self.enviar(&Mensaje::PublicText {
            text: text.trim().to_string(),
        })?;

        Ok(true)
    }

    pub fn iniciar_escucha(&self) -> Result<(), Error> {
        let stream = self
            .stream
            .as_ref()
            .ok_or_else(|| Error::other("El cliente no está conectado"))?
            .try_clone()?;

        thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut mensaje = String::new();

            loop {
                match reader.read_line(&mut mensaje) {
                    Ok(0) => {
                        println!("La conexión con el servidor se cerró.");
                        break;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("Error recibiendo mensajes del servidor: {}", error);
                        break;
                    }
                }

                match traductor::deserializa(&mensaje) {
                    Ok(mensaje) => match mensaje {
                        Mensaje::NewUser { username } => {
                            println!("NEW_USER: \"{}\"", username);
                        }
                        Mensaje::Disconnected { username } => {
                            println!("DISCONNECT: \"{}\"", username);
                        }
                        Mensaje::PublicTextFrom { username, text } => {
                            println!("{}: {}", username, text);
                        }
                        Mensaje::Response { result, .. } => {
                            println!("Respuesta del servidor: {:?}", result);
                        }
                        _ => {}
                    },
                    Err(error) => {
                        eprintln!("Se recibió un mensaje inválido del servidor: {}", error);
                    }
                }
                mensaje.clear();
            }
        });

        Ok(())
    }
}
