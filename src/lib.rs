//! Verificador de integridad — Matías Nicolás Echeveste Cardozo
//!
//! Genera un manifiesto con el resumen SHA-256 de cada archivo de una carpeta y lo compara después con el
//! contenido actual para detectar archivos alterados, faltantes o nuevos.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

pub const VERSION_FORMATO: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entrada {
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifiesto {
    pub formato: u32,
    pub algoritmo: String,
    /// Rutas relativas con separador `/`, ordenadas, para que el manifiesto sea idéntico en cualquier sistema.
    pub archivos: BTreeMap<String, Entrada>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Informe {
    pub intactos: usize,
    pub alterados: Vec<String>,
    pub faltantes: Vec<String>,
    pub nuevos: Vec<String>,
}

impl Informe {
    pub fn integro(&self) -> bool {
        self.alterados.is_empty() && self.faltantes.is_empty() && self.nuevos.is_empty()
    }
}

/// Calcula el SHA-256 de un archivo leyendo en bloques de 64 KiB (memoria constante con archivos grandes).
pub fn resumen_archivo(ruta: &Path) -> Result<Entrada> {
    let archivo =
        File::open(ruta).with_context(|| format!("no se pudo abrir {}", ruta.display()))?;
    let mut lector = BufReader::new(archivo);
    let mut hasher = Sha256::new();
    let mut bloque = vec![0u8; 64 * 1024];
    let mut bytes = 0u64;
    loop {
        let leidos = lector.read(&mut bloque)?;
        if leidos == 0 {
            break;
        }
        hasher.update(&bloque[..leidos]);
        bytes += leidos as u64;
    }
    Ok(Entrada {
        sha256: hex::encode(hasher.finalize()),
        bytes,
    })
}

fn relativa(raiz: &Path, ruta: &Path) -> String {
    ruta.strip_prefix(raiz)
        .unwrap_or(ruta)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Lista los archivos regulares de `raiz`, excluyendo las rutas relativas indicadas.
pub fn listar(raiz: &Path, excluir: &[String]) -> Result<Vec<PathBuf>> {
    let mut archivos = Vec::new();
    for entrada in WalkDir::new(raiz).follow_links(false) {
        let entrada = entrada?;
        if entrada.file_type().is_file() {
            let rel = relativa(raiz, entrada.path());
            if !excluir
                .iter()
                .any(|e| rel == *e || rel.starts_with(&format!("{e}/")))
            {
                archivos.push(entrada.into_path());
            }
        }
    }
    Ok(archivos)
}

/// Genera el manifiesto de una carpeta calculando los resúmenes en paralelo.
pub fn generar(raiz: &Path, excluir: &[String]) -> Result<Manifiesto> {
    let archivos = listar(raiz, excluir)?;
    let entradas: Result<Vec<(String, Entrada)>> = archivos
        .par_iter()
        .map(|ruta| Ok((relativa(raiz, ruta), resumen_archivo(ruta)?)))
        .collect();
    Ok(Manifiesto {
        formato: VERSION_FORMATO,
        algoritmo: "sha256".into(),
        archivos: entradas?.into_iter().collect(),
    })
}

/// Compara un manifiesto con el estado actual de la carpeta.
pub fn verificar(raiz: &Path, manifiesto: &Manifiesto, excluir: &[String]) -> Result<Informe> {
    anyhow::ensure!(
        manifiesto.formato == VERSION_FORMATO,
        "formato de manifiesto no admitido: {}",
        manifiesto.formato
    );
    anyhow::ensure!(
        manifiesto.algoritmo == "sha256",
        "algoritmo no admitido: {}",
        manifiesto.algoritmo
    );
    let actual = generar(raiz, excluir)?;
    let mut informe = Informe::default();
    for (ruta, esperada) in &manifiesto.archivos {
        match actual.archivos.get(ruta) {
            None => informe.faltantes.push(ruta.clone()),
            Some(e) if e != esperada => informe.alterados.push(ruta.clone()),
            Some(_) => informe.intactos += 1,
        }
    }
    informe.nuevos = actual
        .archivos
        .keys()
        .filter(|r| !manifiesto.archivos.contains_key(*r))
        .cloned()
        .collect();
    Ok(informe)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::fs;

    #[test]
    fn resumen_conocido() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("hola.txt");
        fs::write(&ruta, "hola").unwrap();
        let e = resumen_archivo(&ruta).unwrap();
        assert_eq!(
            e.sha256,
            "b221d9dbb083a7f33428d7c2a3c3198ae925614d70210e28716ccaa7cd4ddb79"
        );
        assert_eq!(e.bytes, 4);
    }

    #[test]
    fn rutas_relativas_con_barra() {
        let raiz = Path::new("base");
        assert_eq!(relativa(raiz, &raiz.join("a").join("b.txt")), "a/b.txt");
    }
}
