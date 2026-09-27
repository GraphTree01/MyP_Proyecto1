use serde::{Deserialize, Serialize};

/// Mensajes intercambiados entre clientes y servidor.
///
/// El atributo `type` de cada JSON determina la variante que debe
/// deserializarse.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Mensaje {
    /// Solicita registrar un nombre para la conexión actual.
    #[serde(rename = "IDENTIFY")]
    Identify { username: String },

    /// Respuesta del servidor a una operación del protocolo.
    #[serde(rename = "RESPONSE")]
    Response {
        operation: Operation,
        result: Resultado,
        #[serde(skip_serializing_if = "Option::is_none")]
        extra: Option<String>,
    },

    /// Notifica que un nuevo usuario se registró correctamente.
    #[serde(rename = "NEW_USER")]
    NewUser { username: String },

    /// Texto público enviado por el cliente al servidor.
    #[serde(rename = "PUBLIC_TEXT")]
    PublicText { text: String },

    /// Texto público que el servidor entrega a los demás clientes.
    #[serde(rename = "PUBLIC_TEXT_FROM")]
    PublicTextFrom { username: String, text: String },

    /// Notifica a los demás clientes que un usuario abandonó el chat.
    #[serde(rename = "DISCONNECTED")]
    Disconnected { username: String },
}

/// Operación a la que corresponde una respuesta del servidor.
#[derive(Debug, Serialize, Deserialize)]
pub enum Operation {
    /// Resultado de una solicitud de identificación.
    #[serde(rename = "IDENTIFY")]
    Identify,
    /// Se recibió una operación no válida.
    #[serde(rename = "INVALID")]
    Invalid,
}

/// Resultado de una operación del protocolo.
#[derive(Debug, Serialize, Deserialize)]
pub enum Resultado {
    /// La operación se realizó correctamente.
    #[serde(rename = "SUCCESS")]
    Success,
    /// El nombre solicitado ya está ocupado.
    #[serde(rename = "USER_ALREADY_EXIST")]
    UserAlreadyExist,
    /// El cliente no pudo ser identificado.
    #[serde(rename = "NOT_IDENTIFIED")]
    NotIdentified,
    /// La operación o sus datos no son válidos.
    #[serde(rename = "INVALID")]
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prueba_identify() {
        let mensaje = Mensaje::Identify {
            username: String::from("Kimberly"),
        };

        let json = serde_json::to_string(&mensaje).unwrap();

        assert_eq!(json, r#"{"type":"IDENTIFY","username":"Kimberly"}"#);
    }

    #[test]
    fn prueba_response_success() {
        let mensaje = Mensaje::Response {
            operation: Operation::Identify,
            result: Resultado::Success,
            extra: Some(String::from("Kimberly")),
        };

        let json = serde_json::to_string(&mensaje).unwrap();

        assert_eq!(
            json,
            r#"{"type":"RESPONSE","operation":"IDENTIFY","result":"SUCCESS","extra":"Kimberly"}"#
        );
    }

    #[test]
    fn prueba_response_user_already_exist() {
        let mensaje = Mensaje::Response {
            operation: Operation::Identify,
            result: Resultado::UserAlreadyExist,
            extra: Some(String::from("Kimberly")),
        };

        let json = serde_json::to_string(&mensaje).unwrap();

        assert_eq!(
            json,
            r#"{"type":"RESPONSE","operation":"IDENTIFY","result":"USER_ALREADY_EXIST","extra":"Kimberly"}"#
        );
    }
}
