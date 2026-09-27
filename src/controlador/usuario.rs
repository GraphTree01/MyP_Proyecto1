//! Estado asociado a un usuario autenticado.

use std::net::TcpStream;

/// Usuario registrado en el servidor junto con su canal de salida.
pub struct Usuario {
    /// Nombre único utilizado como clave del diccionario de usuarios.
    pub nombre: String,
    /// Stream usado por el servidor para enviarle mensajes.
    pub stream: TcpStream,
}
