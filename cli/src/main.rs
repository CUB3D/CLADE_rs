pub mod config;

use crate::config::Config as TomlConfig;
use clade::config::CladeConfig;
use clade::{
    DICT_SIZE_BYTES, DICT_SIZE_WORDS, clade_read,
    types::{MemRequest, PdParams},
};
use clap::Parser;
use std::path::Path;

/// Utility for decompressing Qualcomm CLADE compressed modem images
#[derive(clap::Parser, Clone)]
pub struct Args {
    /// The path to a flat binary of the firmware
    pub flat_mem_path: String,

    /// The path to the config file, see ./specs
    pub config_path: String,
}

/// Save an ELF alongside the bin files, for testing with objdump
const OUTPUT_ELF: bool = false;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let cfg = toml::from_str::<TomlConfig>(&std::fs::read_to_string(&args.config_path)?)?;

    for pd in 0..cfg.program_directory.len() {
        let lo_data = decompress(args.clone(), pd, false, &cfg)?;
        std::fs::write(format!("./out_{pd}_low.bin"), &lo_data)?;
        let hi_data = decompress(args.clone(), pd, true, &cfg)?;
        std::fs::write(format!("./out_{pd}_high.bin"), &hi_data)?;

        if OUTPUT_ELF {
            use faerie::{ArtifactBuilder, Decl};
            {
                let file = std::fs::File::create(Path::new("./out_lo.bin.elf"))?;
                let mut obj = ArtifactBuilder::new(target_lexicon::Triple::host())
                    .name("out".to_owned())
                    .finish();

                obj.declarations([("deadbeef", Decl::function().into())].iter().cloned())?;

                obj.define("deadbeef", lo_data.clone())?;

                obj.write(file)?;
            }
            {
                let file = std::fs::File::create(Path::new("./out_hi.bin.elf"))?;
                let mut obj = ArtifactBuilder::new(target_lexicon::Triple::host())
                    .name("out".to_owned())
                    .finish();

                obj.declarations([("deadbeef", Decl::function().into())].iter().cloned())?;

                obj.define("deadbeef", hi_data.clone())?;

                obj.write(file)?;
            }
        }
    }

    Ok(())
}

pub fn decompress(args: Args, pd: usize, high: bool, cfg: &TomlConfig) -> anyhow::Result<Vec<u8>> {
    let flat = std::fs::read(args.flat_mem_path)?;

    let mut dicts = [[0u32; DICT_SIZE_WORDS]; 3];

    for (idx, off) in cfg.dictionaries.iter().enumerate() {
        let dat: &[u8] = &flat[*off as usize..][..DICT_SIZE_BYTES];
        let dat = bytemuck::cast_slice::<u8, u32>(dat);
        dicts[idx] = dat.try_into()?;
    }

    let pd_params = cfg
        .program_directory
        .iter()
        .map(|p| PdParams {
            comp: p.comp,
            exc_hi: p.exc_hi,
            exc_lo: p.exc_lo,
            exc_lo_small: p.exc_lo_small,
            ..Default::default()
        })
        .collect::<Vec<_>>();

    let region = pd_params[0].comp;
    let mut config = CladeConfig {
        region,
        pd_params,
        dicts,
    };
    config.init();

    let addr = if high {
        config.pd_params[pd].region_hi
    } else {
        config.pd_params[pd].region_lo
    };
    let len = if high {
        cfg.size_high
    } else {
        cfg.size_low
    };
    let req = MemRequest {
        addr,
        len,
    };

    let out = clade_read(&config, &[req], &flat)?;
    Ok(out.into_iter().next().unwrap())
}
