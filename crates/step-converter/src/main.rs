use std::{env, fs::File, process::ExitCode};

use cadrum::{Solid, Tessellation};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let input_path = args
        .next()
        .ok_or_else(|| "missing STEP input path".to_owned())?;
    let output_path = args
        .next()
        .ok_or_else(|| "missing STL output path".to_owned())?;

    if args.next().is_some() {
        return Err("usage: space-thumbnails-step-converter <input.step> <output.stl>".to_owned());
    }

    let mut input = File::open(&input_path)
        .map_err(|err| format!("failed to open STEP input {:?}: {err}", input_path))?;
    let solids = Solid::read_step(&mut input)
        .map_err(|err| format!("failed to read STEP input {:?}: {err}", input_path))?;
    if solids.is_empty() {
        return Err(format!(
            "STEP input {:?} did not contain solids",
            input_path
        ));
    }

    let mesh = Solid::mesh(
        &solids,
        Tessellation {
            deflection_linear: 0.1,
            relative_linear: false,
            ..Default::default()
        },
    )
    .map_err(|err| format!("failed to tessellate STEP input {:?}: {err}", input_path))?;

    let mut output = File::create(&output_path)
        .map_err(|err| format!("failed to create STL output {:?}: {err}", output_path))?;
    mesh.write_stl(&mut output)
        .map_err(|err| format!("failed to write STL output {:?}: {err}", output_path))?;

    Ok(())
}
