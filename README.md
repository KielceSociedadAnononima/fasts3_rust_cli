# ⚡ FastS3 Rust Client (`fasts3_rust_cli`)

Librería cliente asíncrona completa (SDK) en **Rust** para consumir el 100% de la API REST del servidor de almacenamiento **FastS3**.

## ✨ Características

- **Cobertura Total de Endpoints**: Autenticación y Bootstrap, API Keys, Usuarios, Matriz de Permisos por Bucket, Volúmenes Multi-Disco, Buckets, Archivos/Streams, Miniaturas y AutoVacuum.
- **Todos los Métodos de Autenticación**:
  - `Credentials::basic(user, pass)` (`Authorization: Basic ...`)
  - `Credentials::bearer(token)` (`Authorization: Bearer ...` para Bootstrap Token o API Key)
  - `Credentials::api_key(key)` (`X-API-Key: fs3_...`)
  - `Credentials::fasts3_token(token)` (`X-FastS3-Token: ...`)
  - `Credentials::custom_headers(user, pass)` (`X-FastS3-User` + `X-FastS3-Pass`)
  - `Credentials::None` (acceso anónimo a buckets públicos)
- **Streaming Asíncrono sin Buffering en RAM**: Subida (`upload_file`, `upload_stream`) y descarga (`download_to_path`, `download_to_writer`) por chunks usando Tokio.
- **Operaciones de Carpetas / Prefijos**: `upload_directory`, `download_prefix_to_dir` y `delete_prefix`.
- **Generadores de URLs Directas**: Construcción de enlaces directos y miniaturas con autenticación embebida por query string (`?token=...`, `?api_key=...`, `?auth=...`).
- **Thread-Safe (`Clone + Send + Sync`)**: Comparte una única instancia de `FastS3Client` entre tareas de Tokio con actualización dinámica de credenciales.

---

## 📦 Instalación

En tu `Cargo.toml`:

```toml
[dependencies]
fasts3_rust_cli = { path = "../fasts3_rust_cli" }
tokio = { version = "1", features = ["full"] }
```

---

## 🚀 Uso Rápido

```rust
use fasts3_rust_cli::{
    CreateBucketRequest, CreateDiskRequest, CreateUserRequest, FastS3Client,
    FileListFilter, SetPermissionRequest, ThumbnailOptions,
};

#[tokio::main]
async fn main() -> fasts3_rust_cli::Result<()> {
    // 1. Conectar usando Basic Auth, Bearer Token o API Key
    let client = FastS3Client::with_basic_auth("http://localhost:8080", "admin", "secret")?;

    // Verificar sesión actual
    let session = client.verify_auth().await?;
    println!("Autenticado como {} (admin={})", session.username, session.is_admin);

    // 2. Crear un bucket con cuota de 100 MB
    client
        .create_bucket(
            &CreateBucketRequest::new("documentos")
                .public(false)
                .quota(100 * 1024 * 1024),
        )
        .await?;

    // 3. Subir datos en memoria o archivos locales por streaming
    client
        .upload_bytes(
            "documentos",
            "reportes/2026/resumen.txt",
            "Contenido del reporte",
            Some("text/plain"),
        )
        .await?;

    // 4. Listar archivos y detalles enriquecidos
    let filter = FileListFilter::new().with_subpath("reportes/2026");
    let details = client.list_file_details("documentos", Some(&filter)).await?;
    for f in details {
        println!("{} ({} bytes, {})", f.key, f.size, f.mime_type);
    }

    // 5. Descargar archivo
    let content = client.download_bytes("documentos", "reportes/2026/resumen.txt").await?;
    println!("Descargado: {} bytes", content.len());

    // 6. Gestión de Usuarios, Permisos y API Keys
    let worker = client
        .create_user(&CreateUserRequest::new("worker", "ClaveSegura123!", false))
        .await?;

    client
        .set_permission(&SetPermissionRequest {
            user_id: worker.id,
            bucket_name: "documentos".into(),
            can_read: true,
            can_write: true,
            can_delete: false,
        })
        .await?;

    let api_key = client.create_api_key("Worker Service Key", Some(worker.id)).await?;
    println!("API Key generada: {}", api_key.api_key);

    // 7. Ejecutar mantenimiento AutoVacuum
    let stats = client.trigger_vacuum().await?;
    println!("Buckets reconciliados: {}", stats.buckets_reconciled);

    Ok(())
}
```

---

## 📚 Resumen de Métodos Disponibles

| Categoría | Métodos de `FastS3Client` |
|---|---|
| **Autenticación & Bootstrap** | `verify_auth`, `bootstrap_status`, `bootstrap_login`, `bootstrap_login_and_authenticate`, `bootstrap_setup`, `bootstrap_setup_and_authenticate`, `login`, `login_and_authenticate`, `change_password`, `change_password_and_authenticate`, `get_bootstrap_token` |
| **API Keys** | `list_api_keys`, `create_api_key`, `delete_api_key` |
| **Usuarios** | `list_users`, `get_user_by_username`, `create_user`, `update_user`, `delete_user` |
| **Permisos RBAC** | `list_permissions`, `set_permission`, `delete_permission`, `get_bucket_permissions`, `delete_bucket_permission` |
| **Volúmenes Multi-Disco** | `list_disks`, `add_disk`, `delete_disk` |
| **Buckets** | `list_buckets`, `get_bucket_by_name`, `create_bucket`, `update_bucket`, `delete_bucket`, `delete_bucket_by_name` |
| **Archivos & Miniaturas** | `list_files`, `list_file_details`, `upload_bytes`, `upload_stream`, `upload_file`, `upload_json`, `upload_json_bytes`, `download_file`, `download_bytes`, `download_with_options`, `download_to_writer`, `download_to_path`, `get_thumbnail`, `get_thumbnail_alt`, `save_thumbnail_to_path`, `delete_file`, `upload_directory`, `download_prefix_to_dir`, `delete_prefix`, `file_url`, `thumbnail_url` |
| **AutoVacuum** | `vacuum_status`, `trigger_vacuum` |

---

## 🧪 Ejecutar Pruebas

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```
