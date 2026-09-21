use std::io::{BufRead, BufReader, Write, Error};
use std::net::TcpStream;

use crate::controlador::protocolo::{Mensaje, Operation, Resultado};
use crate::controlador::traductor;

pub struct Manejador {
    reader: BufReader<TcpStream>,
}

impl Manejador {
    pub fn nuevo(stream: TcpStream) -> Self {
        Self {
            reader: BufReader::new(stream),
        }
    }

    pub fn enviar(&mut self, mensaje: &Mensaje) -> Result<(), Error> {
        
        let mut json = traductor::serializa(mensaje)?;
        json.push('\n');

        self.reader
            .get_mut()
            .write_all(json.as_bytes())?;

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

                let respuesta = Mensaje::Response {
                    operation: Operation::Identify,
                    result: Resultado::Success,
                    extra: Some(String::from(username)),
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
