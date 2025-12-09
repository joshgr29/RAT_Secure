use serde::{Deserialize, Serialize};

/// C'est ici qu'on définit tout ce que nos machines peuvent se dire.
/// Le "derive" permet à Rust de convertir automatiquement ces données en JSON.
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    /// Le Serveur demande à l'Agent d'exécuter une commande (ex: "dir", "whoami")
    Command(String),

    /// L'Agent renvoie le résultat de la commande au Serveur
    Output(String),

    /// L'Agent signale une erreur (ex: commande inconnue)
    Error(String),
    
    File { name: String, content: Vec<u8> },
}

