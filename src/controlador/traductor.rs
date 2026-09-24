use super::protocolo::Mensaje;

pub fn serializa(mensaje: &Mensaje) -> Result<String, serde_json::Error> {
    serde_json::to_string(mensaje)
}

pub fn deserializa(json: &str) -> Result<Mensaje, serde_json::Error> {
    serde_json::from_str(json)
}
