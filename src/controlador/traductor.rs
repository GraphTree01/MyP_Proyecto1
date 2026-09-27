//! Conversión entre mensajes del protocolo y texto JSON.

use super::protocolo::Mensaje;

/// Serializa un mensaje para enviarlo por la conexión TCP.
pub fn serializa(mensaje: &Mensaje) -> Result<String, serde_json::Error> {
    serde_json::to_string(mensaje)
}

/// Deserializa una línea JSON recibida desde una conexión TCP.
pub fn deserializa(json: &str) -> Result<Mensaje, serde_json::Error> {
    serde_json::from_str(json)
}
