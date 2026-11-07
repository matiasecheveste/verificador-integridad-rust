//! Verificador de integridad — Matías Nicolás Echeveste Cardozo
//! Pruebas de integración: biblioteca y binario.

use std::fs;
use std::process::Command;

use verificador_integridad::{generar, verificar};

fn carpeta_de_ejemplo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("docs/anexos")).unwrap();
    fs::write(dir.path().join("informe.txt"), "versión final").unwrap();
    fs::write(dir.path().join("docs/acta.md"), "# Acta").unwrap();
    fs::write(dir.path().join("docs/anexos/datos.csv"), "a;b\n1;2\n").unwrap();
    dir
}

#[test]
fn detecta_alterados_faltantes_y_nuevos() {
    let dir = carpeta_de_ejemplo();
    let m = generar(dir.path(), &[]).unwrap();
    assert_eq!(m.archivos.len(), 3);
    assert!(m.archivos.contains_key("docs/anexos/datos.csv"));
    assert!(verificar(dir.path(), &m, &[]).unwrap().integro());

    fs::write(dir.path().join("informe.txt"), "versión modificada").unwrap();
    fs::remove_file(dir.path().join("docs/acta.md")).unwrap();
    fs::write(dir.path().join("intruso.exe"), [0u8; 16]).unwrap();

    let informe = verificar(dir.path(), &m, &[]).unwrap();
    assert_eq!(informe.alterados, vec!["informe.txt"]);
    assert_eq!(informe.faltantes, vec!["docs/acta.md"]);
    assert_eq!(informe.nuevos, vec!["intruso.exe"]);
    assert_eq!(informe.intactos, 1);
}

#[test]
fn respeta_exclusiones() {
    let dir = carpeta_de_ejemplo();
    let m = generar(dir.path(), &["docs".into()]).unwrap();
    assert_eq!(m.archivos.keys().collect::<Vec<_>>(), vec!["informe.txt"]);
}

#[test]
fn rechaza_manifiestos_de_otro_formato() {
    let dir = carpeta_de_ejemplo();
    let mut m = generar(dir.path(), &[]).unwrap();
    m.algoritmo = "md5".into();
    assert!(verificar(dir.path(), &m, &[]).is_err());
}

#[test]
fn binario_devuelve_codigos_de_salida() {
    let dir = carpeta_de_ejemplo();
    let bin = env!("CARGO_BIN_EXE_verificar");
    let manifiesto = dir.path().join("manifiesto.json");

    let generar = Command::new(bin)
        .args(["generar", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        generar.status.success(),
        "{}",
        String::from_utf8_lossy(&generar.stderr)
    );
    assert!(manifiesto.exists());

    let ok = Command::new(bin)
        .args(["comprobar", ".", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        ok.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&ok.stdout)
    );

    fs::write(dir.path().join("docs/acta.md"), "# Acta corregida").unwrap();
    let mal = Command::new(bin)
        .args(["comprobar", ".", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(mal.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&mal.stdout).unwrap();
    assert_eq!(json["alterados"][0], "docs/acta.md");

    let error = Command::new(bin)
        .args(["comprobar", ".", "-m", "no-existe.json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(error.status.code(), Some(2));
}
