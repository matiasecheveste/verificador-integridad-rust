//! Verificador de integridad — Matías Nicolás Echeveste Cardozo
//! Interfaz de línea de comandos.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use verificador_integridad::{generar, verificar, Manifiesto};

#[derive(Parser)]
#[command(
    name = "verificar",
    version,
    about = "Manifiestos SHA-256 para verificar la integridad de una carpeta"
)]
struct Cli {
    #[command(subcommand)]
    orden: Orden,
}

#[derive(Subcommand)]
enum Orden {
    /// Calcula el SHA-256 de cada archivo y guarda el manifiesto en formato JSON
    Generar {
        carpeta: PathBuf,
        #[arg(short, long, default_value = "manifiesto.json")]
        salida: PathBuf,
        /// Rutas relativas a excluir (puede repetirse)
        #[arg(short, long)]
        excluir: Vec<String>,
    },
    /// Compara la carpeta con un manifiesto; termina con código 1 si hay diferencias
    Comprobar {
        carpeta: PathBuf,
        #[arg(short, long, default_value = "manifiesto.json")]
        manifiesto: PathBuf,
        #[arg(short, long)]
        excluir: Vec<String>,
        /// Muestra el resultado en JSON (útil en integración continua)
        #[arg(long)]
        json: bool,
    },
}

/// Si el manifiesto se guarda dentro de la carpeta analizada, lo excluye para que no se verifique a sí mismo.
fn excluir_si_esta_dentro(carpeta: &Path, archivo: &Path, excluir: &mut Vec<String>) {
    let padre = archivo
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let (Ok(base), Ok(padre), Some(nombre)) = (
        fs::canonicalize(carpeta),
        fs::canonicalize(padre),
        archivo.file_name(),
    ) else {
        return;
    };
    if let Ok(rel) = padre.join(nombre).strip_prefix(&base) {
        let partes: Vec<_> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect();
        excluir.push(partes.join("/"));
    }
}

fn ejecutar() -> Result<bool> {
    match Cli::parse().orden {
        Orden::Generar {
            carpeta,
            salida,
            mut excluir,
        } => {
            excluir_si_esta_dentro(&carpeta, &salida, &mut excluir);
            let m = generar(&carpeta, &excluir)?;
            fs::write(&salida, serde_json::to_string_pretty(&m)? + "\n")
                .with_context(|| format!("no se pudo escribir {}", salida.display()))?;
            println!(
                "Manifiesto con {} guardado en {}",
                cantidad(m.archivos.len(), "archivo", "archivos"),
                salida.display()
            );
            Ok(true)
        }
        Orden::Comprobar {
            carpeta,
            manifiesto,
            mut excluir,
            json,
        } => {
            let texto = fs::read_to_string(&manifiesto)
                .with_context(|| format!("no se pudo leer {}", manifiesto.display()))?;
            let m: Manifiesto = serde_json::from_str(&texto).context("manifiesto inválido")?;
            excluir_si_esta_dentro(&carpeta, &manifiesto, &mut excluir);
            let informe = verificar(&carpeta, &m, &excluir)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "integro": informe.integro(),
                        "intactos": informe.intactos,
                        "alterados": informe.alterados,
                        "faltantes": informe.faltantes,
                        "nuevos": informe.nuevos,
                    })
                );
            } else {
                for r in &informe.alterados {
                    println!("ALTERADO  {r}");
                }
                for r in &informe.faltantes {
                    println!("FALTANTE  {r}");
                }
                for r in &informe.nuevos {
                    println!("NUEVO     {r}");
                }
                println!(
                    "{}, {}, {}, {}",
                    cantidad(informe.intactos, "intacto", "intactos"),
                    cantidad(informe.alterados.len(), "alterado", "alterados"),
                    cantidad(informe.faltantes.len(), "faltante", "faltantes"),
                    cantidad(informe.nuevos.len(), "nuevo", "nuevos")
                );
            }
            Ok(informe.integro())
        }
    }
}

/// Cantidad con el sustantivo en singular o plural: «1 archivo», «3 archivos».
fn cantidad(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n == 1 { singular } else { plural })
}

fn main() -> ExitCode {
    match ejecutar() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}
