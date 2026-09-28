use std::collections::HashSet;

/// Sala de chat única que puede generar un usuario llamada: cuarto.
pub struct Cuarto {
    /// Nombre único del cuarto.
    pub nombre: String,
    /// Usuarios que pertenecen actualmente al cuarto.
    pub miembros: HashSet<String>,
    /// Usuarios invitados que todavía no pertenecen al cuarto.
    pub invitados: HashSet<String>,
}

impl Cuarto {
    /// Comprueba que el nombre no esté vacío y tenga como máximo 16 caracteres.
    pub fn nombre_valido(nombre: &str) -> bool {
        !nombre.is_empty() && nombre.chars().count() <= 16
    }

    /// Crea una cuarto incorporando inmediatamente a su primer usuario.
    pub fn nuevo(nombre: String, usuario: String) -> Self {
        let mut miembros = HashSet::new();
        miembros.insert(usuario);

        Self {
            nombre,
            miembros,
            invitados: HashSet::new(),
        }
    }

    /// Comprueba si el cuarto no tiene miembros.
    pub fn esta_vacio(&self) -> bool {
        self.miembros.is_empty()
    }
}
