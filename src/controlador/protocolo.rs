use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

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

    /// Inicializa o cambia el status de los usuarios.
    #[serde(rename = "STATUS")]
    Status { status: Status },

    /// Notifica que un usuario a cambiado su status.
    #[serde(rename = "NEW_STATUS")]
    NewStatus { username: String, status: Status },

    /// Texto público enviado por el cliente al servidor.
    #[serde(rename = "PUBLIC_TEXT")]
    PublicText { text: String },

    /// Texto público que el servidor entrega a los demás clientes.
    #[serde(rename = "PUBLIC_TEXT_FROM")]
    PublicTextFrom { username: String, text: String },

    /// Texto privado enviado por el cliente al servidor.
    #[serde(rename = "TEXT")]
    PrivateText { username: String, text: String },

    /// Texto privado que el servidor entrega a un cliente específico.
    #[serde(rename = "TEXT_FROM")]
    PrivateTextFrom { username: String, text: String },

    /// Notifica a los demás clientes que un usuario abandonó el chat.
    #[serde(rename = "DISCONNECTED")]
    Disconnected { username: String },

    /// Solicita cerrar la conexión actual sin responder al cliente.
    #[serde(rename = "DISCONNECT")]
    Disconnect,

    /// Solicita la lista de usuarios en el chat.
    #[serde(rename = "USERS")]
    Users,

    /// Respuesta del servidor al cliente que pide la lista de usuarios.
    #[serde(rename = "USER_LIST")]
    UserList { users: HashMap<String, Status> },

    #[serde(rename = "NEW_ROOM")]
    NewRoom { roomname: String },

    #[serde(rename = "INVITE")]
    Invite {
        roomname: String,
        usernames: Vec<String>,
    },

    /// Notifica a un usuario que fue invitado a una sala.
    #[serde(rename = "INVITATION")]
    Invitation { username: String, roomname: String },
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
    /// Resultado si el usuario destinatario no existe.
    #[serde(rename = "TEXT")]
    Text,

    #[serde(rename = "NEW_ROOM")]
    NewRoom,

    #[serde(rename = "INVITE")]
    Invite,
}

/// Estado visible de un usuario conectado.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Status {
    /// El usuario está disponible.
    #[serde(rename = "ACTIVE")]
    Active,

    /// El usuario está temporalmente ausente.
    #[serde(rename = "AWAY")]
    Away,

    /// El usuario está ocupado.
    #[serde(rename = "BUSY")]
    Busy,
}

impl FromStr for Status {
    type Err = ();

    fn from_str(status: &str) -> Result<Self, Self::Err> {
        match status.to_ascii_uppercase().as_str() {
            "ACTIVE" => Ok(Self::Active),
            "AWAY" => Ok(Self::Away),
            "BUSY" => Ok(Self::Busy),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            Self::Active => "ACTIVE",
            Self::Away => "AWAY",
            Self::Busy => "BUSY",
        };

        formatter.write_str(status)
    }
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
    /// El cliente destinatario no existe.
    #[serde(rename = "NO_SUCH_USER")]
    NoSuchUser,

    #[serde(rename = "ROOM_ALREADY_EXISTS")]
    RoomAlreadyExists,

    #[serde(rename = "NO_SUCH_ROOM")]
    NoSuchRoom,
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
