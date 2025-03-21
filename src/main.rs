use clap::{Parser, Subcommand};
use probe_rs::{flashing, MemoryInterface, Permissions, Session};
use std::result;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    option: ChipOption,
    #[arg(short, long)]
    chip: Option<String>,
}

#[derive(Subcommand)]
enum ChipOption {
    /// Read or write NDBANK bit
    NDBANK { value: Option<bool> },
}

impl std::fmt::Debug for ChipOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NDBANK { value } => f.write_fmt(format_args!("NDBANK is {}", value.unwrap())),
        }
    }
}

fn get_session(chip_name: Option<String>) -> Result<probe_rs::Session, probe_rs::Error> {
    if let Some(chip_name) = chip_name {
        let target = probe_rs::config::get_target_by_name(chip_name)?;
        return Ok(Session::auto_attach(target, Permissions::default())?);
    }

    Ok(Session::auto_attach(
        probe_rs::config::TargetSelector::Auto,
        Permissions::default(),
    )?)
}

// Tried to use the stm32_metapac::common::Reg as a wrapper to a copied value. Not sure if this is a good approach...
fn read_reg_from_mem<T: Copy, A: stm32_metapac::common::Access>(
    core: &mut probe_rs::Core,
    reg: stm32_metapac::common::Reg<T, A>,
) -> Result<stm32_metapac::common::Reg<u32, A>, probe_rs::Error> {
    let mut bytes = [0u8; 4];
    core.read_mem_32bit(reg.as_ptr() as u64, &mut bytes)?;
    let my_data = Box::into_raw(Box::new(u32::from_le_bytes(bytes)));
    unsafe { Ok(stm32_metapac::common::Reg::<u32, A>::from_ptr(my_data)) }
}

fn read_u32_from_mem(core: &mut probe_rs::Core, address: u64) -> Result<u32, probe_rs::Error> {
    let mut bytes = [0u8; 4];
    core.read_mem_32bit(address, &mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn reg_to_addr<T: Copy, A: stm32_metapac::common::Access>(
    reg: &stm32_metapac::common::Reg<T, A>,
) -> u64 {
    reg.as_ptr() as u64
}

fn read_option_byte(
    chip_name: Option<String>,
    option: ChipOption,
) -> Result<ChipOption, probe_rs::Error> {
    let mut session = get_session(chip_name)?;
    let mut core = session.core(0)?;

    let optcr = read_u32_from_mem(&mut core, reg_to_addr(&stm32_metapac::FLASH.optcr()))?;
    println!("Read OPTCR 0x{:08x}", optcr);

    match option {
        ChipOption::NDBANK { .. } => Ok(ChipOption::NDBANK {
            value: Some((optcr >> FLASH_OPTCR_NDBANK_BIT) & 0x01 != 0),
        }),
    }
}

#[derive(Debug)]
enum Error {
    ProbeRsError(probe_rs::Error),
    ProbeRsFlashError(probe_rs::flashing::FlashError),
}

impl From<probe_rs::Error> for Error {
    fn from(value: probe_rs::Error) -> Self {
        Error::ProbeRsError(value)
    }
}

impl From<probe_rs::flashing::FlashError> for Error {
    fn from(value: probe_rs::flashing::FlashError) -> Self {
        Error::ProbeRsFlashError(value)
    }
}

struct FlashCr {
    optcr: u32,
    optcr1: u32,
}

fn read_flash_cr_regs(session: &mut probe_rs::Session) -> Result<FlashCr, Error> {
    let mut core = session.core(0)?;
    let optcr = read_u32_from_mem(&mut core, reg_to_addr(&stm32_metapac::FLASH.optcr()))?;
    let optcr1 = read_u32_from_mem(&mut core, reg_to_addr(&stm32_metapac::FLASH.optcr1()))?;

    Ok(FlashCr {
        optcr: optcr,
        optcr1: optcr1,
    })
}

const FLASH_OPTCR_NDBANK_BIT: u8 = 29;

fn write_ndbank_bit(chip_name: Option<String>, value: bool) -> Result<(), Error> {
    let mut session = get_session(chip_name)?;
    let mut regs = read_flash_cr_regs(&mut session)?;
    let target = session.target();
    let mut flash_loader = target.flash_loader();

    set_bit_value(&mut regs.optcr, FLASH_OPTCR_NDBANK_BIT, value);

    const FLASH_ALGORITHM_OPTCR_ADDR: u64 = 0x1FFF0000;
    const FLASH_ALGORITHM_OPTCR1_ADDR: u64 = 0x1FFF0004;

    flash_loader.add_data(FLASH_ALGORITHM_OPTCR_ADDR, &regs.optcr.to_le_bytes())?;
    flash_loader.add_data(FLASH_ALGORITHM_OPTCR1_ADDR, &regs.optcr1.to_le_bytes())?;
    flash_loader.commit(&mut session, flashing::DownloadOptions::default())?;

    Ok(())
}

fn set_bit_value(x: &mut u32, bit: u8, value: bool) {
    if value {
        *x |= 1 << bit
    } else {
        *x &= !(1 << bit)
    }
}

fn main() -> Result<(), Error> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.option {
        ChipOption::NDBANK { value } => {
            if let Some(value) = value {
                write_ndbank_bit(cli.chip, value)?;
            } else {
                println!("{:?}", read_option_byte(cli.chip, cli.option)?);
            }
        }
    }

    Ok(())
}
