use std::{env, io, path::PathBuf};

use clap::Parser;
use fuser::MountOption;
use tracing::{debug, error, info};

use drive::{AliyunDrive, DriveConfig};
use vfs::AliyunDriveFileSystem;

mod drive;
mod vfs;

#[derive(Parser, Debug)]
#[clap(name = "aliyundrive-fuse", about, version, author)]
struct Opt {
    /// Mount point
    #[clap(parse(from_os_str))]
    path: PathBuf,
    /// Aliyun drive refresh token
    #[clap(short, long, env = "REFRESH_TOKEN")]
    refresh_token: String,
    /// Directory entries cache size
    #[clap(long, default_value = "1000")]
    cache_size: usize,
    /// Directory entries cache expiration time in seconds
    #[clap(long, default_value = "600")]
    cache_ttl: u64,
    /// Root directory path
    #[clap(long, default_value = "/")]
    root: String,
    /// Working directory, refresh_token will be stored in there if specified
    #[clap(short = 'w', long)]
    workdir: Option<PathBuf>,
    /// Delete file permanently instead of trashing it
    #[clap(long, conflicts_with = "domain-id")]
    no_trash: bool,
    /// Aliyun PDS domain id
    #[clap(long)]
    domain_id: Option<String>,
    /// Enable read only mode
    #[clap(long)]
    read_only: bool,
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "native-tls-vendored")]
    openssl_probe::init_ssl_cert_env_vars();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "aliyundrive_fuse=info");
    }
    tracing_subscriber::fmt::init();

    let opt = Opt::parse();
    let (drive_config, no_trash) = if let Some(domain_id) = opt.domain_id {
        (
            DriveConfig {
                api_base_url: format!("https://{}.api.aliyunpds.com", domain_id),
                refresh_token_url: format!(
                    "https://{}.auth.aliyunpds.com/v2/account/token",
                    domain_id
                ),
                workdir: opt.workdir,
                app_id: Some("BasicUI".to_string()),
            },
            true, // PDS doesn't have trash support
        )
    } else {
        (
            DriveConfig {
                api_base_url: "https://api.aliyundrive.com".to_string(),
                refresh_token_url: "https://websv.aliyundrive.com/token/refresh".to_string(),
                workdir: opt.workdir,
                app_id: None,
            },
            opt.no_trash,
        )
    };
    let drive = AliyunDrive::new(drive_config, opt.refresh_token).map_err(|_| {
        io::Error::new(io::ErrorKind::Other, "initialize aliyundrive client failed")
    })?;

    let vfs = AliyunDriveFileSystem::new(drive);
    fuser::mount2(vfs, opt.path, &[MountOption::AutoUnmount])?;
    Ok(())
}