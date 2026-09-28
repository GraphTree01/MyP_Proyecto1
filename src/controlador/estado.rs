//! Estado compartido que administra el servidor.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::controlador::cuarto::Cuarto;
use crate::controlador::usuario::Usuario;

/// Registro único de usuarios y cuartos del servidor.
pub struct EstadoServidor {
    pub usuarios: HashMap<String, Usuario>,
    pub cuartos: HashMap<String, Cuarto>,
}

impl EstadoServidor {
    /// Crea un estado sin usuarios ni cuartos.
    pub fn nuevo() -> Self {
        Self {
            usuarios: HashMap::new(),
            cuartos: HashMap::new(),
        }
    }
}

/// Estado del servidor compartido entre los hilos de conexión.
pub type EstadoCompartido = Arc<Mutex<EstadoServidor>>;
