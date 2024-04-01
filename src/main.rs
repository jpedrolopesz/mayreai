use async_openai::types::{
    CreateAssistantRequestArgs, CreateMessageRequestArgs, CreateRunRequestArgs,
    CreateThreadRequestArgs, MessageContent, RunStatus,
};
use async_openai::types::{CreateSpeechRequestArgs, SpeechModel, Voice};
use async_openai::{types::CreateTranscriptionRequestArgs, Client};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::error::Error;
use std::io;
use std::{env, fs};
use std::{fs::File, sync::mpsc::channel, time::Duration};
use tokio;

mod controllers;
mod router;

struct Curriculo {
    nome: String,
    habilidades: String,
}

struct Empresa {
    nome_empresa: String,
    vaga: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OK");

    transcribe_audio().await?;

    Ok(())
}

fn coletar_informacoes_curriculo() -> Result<(Curriculo, Empresa), Box<dyn Error>> {
    let mut nome_canditato = String::new();
    println!("Digite o seu nome:");
    io::stdin().read_line(&mut nome_canditato)?;

    let mut habilidades = String::new();
    println!("Fale sobre suas habilidades:");
    io::stdin().read_line(&mut habilidades)?;

    let mut nome_empresa = String::new();
    println!("Digite o nome da empresa:");
    io::stdin().read_line(&mut nome_empresa)?;

    let mut vaga = String::new();
    println!("Digite sobre a vaga:");
    io::stdin().read_line(&mut vaga)?;

    let mut start = Vec::new();
    loop {
        println!("Podemos começar? Ou se deseja dexistir me diga 'fim' para terminar: ");

        let mut end = String::new();
        io::stdin().read_line(&mut end)?;
        if end.trim() == "fim" {
            break;
        }
        start.push(end.trim().to_string());
    }

    Ok((
        Curriculo {
            nome: nome_canditato,
            habilidades,
        },
        Empresa { nome_empresa, vaga },
    ))
}

fn criar_summary(curriculo: &Curriculo, empresa: &Empresa, intern: &str) -> String {
    format!(
        "Candidato: {}\nHabilidades: {}\nVaga Desejada: {} na empresa {}.\nGerenciado por estagiario: {}",
        curriculo.nome, curriculo.habilidades, empresa.vaga, empresa.nome_empresa, intern
    )
}

async fn recruiter(summary: &str) -> Result<(), Box<dyn Error>> {
    let query = [("limit", "1")]; //limit the list responses to 1 message

    //create a client
    let client = Client::new();

    //create a thread for the conversation
    let thread_request = CreateThreadRequestArgs::default().build()?;
    let thread = client.threads().create(thread_request.clone()).await?;

    let path = "/Users/jpedrolopesz/Arsenal/mayreai/src/instructions/inst.md";
    let file_content = fs::read_to_string(path)?;

    let mut instructions = format!("{}\n\n{}", file_content, summary);

    io::stdin().read_line(&mut instructions)?;

    println!("{}", instructions);

    //create the assistant
    let assistant_request = CreateAssistantRequestArgs::default()
        .instructions(instructions)
        .model("gpt-3.5-turbo-1106")
        .build()?;
    let assistant = client.assistants().create(assistant_request).await?;
    //get the id of the assistant
    let assistant_id = &assistant.id;

    loop {
        println!("--- How can I help you?");
        //get user input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        //break out of the loop if the user enters exit()
        if input.trim() == "exit()" {
            break;
        }

        //create a message for the thread
        let message = CreateMessageRequestArgs::default()
            .role("user")
            .content(input.clone())
            .build()?;

        //attach message to the thread
        let _message_obj = client
            .threads()
            .messages(&thread.id)
            .create(message)
            .await?;

        //create a run for the thread
        let run_request = CreateRunRequestArgs::default()
            .assistant_id(assistant_id)
            .build()?;
        let run = client
            .threads()
            .runs(&thread.id)
            .create(run_request)
            .await?;

        while let Ok(run) = client.threads().runs(&thread.id).retrieve(&run.id).await {
            match run.status {
                RunStatus::Completed => {
                    let response = client.threads().messages(&thread.id).list(&query).await?;
                    if let Some(message) = response.data.get(0) {
                        let content = message.content.get(0).unwrap();
                        let text = if let MessageContent::Text(text) = content {
                            &text.text.value
                        } else {
                            "Formato de mensagem não suportado."
                        };
                        println!("--- Resposta: {}", text);
                    }
                    break;
                }
                RunStatus::Failed | RunStatus::Cancelled | RunStatus::Expired => {
                    println!("--- A execução falhou ou foi cancelada.");
                    break;
                }
                _ => std::thread::sleep(std::time::Duration::from_secs(1)),
            }
        }
    }

    //once we have broken from the main loop we can delete the assistant and thread
    client.assistants().delete(assistant_id).await?;
    client.threads().delete(&thread.id).await?;

    Ok(())
}

async fn transcribe_audio() -> Result<String, Box<dyn Error>> {
    let _api_key = env::var("OPENAI_API_KEY")?;

    let client = Client::new();
    let request = CreateTranscriptionRequestArgs::default()
        .file("/Users/jpedrolopesz/Arsenal/mayreai/src/audio.m4a")
        .model("whisper-1")
        .build()?;

    let response = client.audio().transcribe(request).await?;

    println!("{}", response.text);

    Ok(response.text)
}

async fn record_audio(duration: u64) -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;
    let config = device.default_input_config()?;
    let (sender, receiver) = channel();
    let sample_format = config.sample_format();
    let sample_rate = config.sample_rate().0 as u32;
    let channels = config.channels();

    let spec = WavSpec {
        channels: channels,
        sample_rate: sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::new(File::create("recording.wav")?, spec)?;

    while let Ok(sample) = receiver.try_recv() {
        writer.write_sample(sample)?;
    }

    let err_fn = |err| eprintln!("Error: {:?}", err);
    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for &sample in data.iter() {
                    let sample = (sample * i16::MAX as f32) as i16;
                    sender.send(sample).expect("Send error");
                }
            },
            err_fn,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                for &sample in data.iter() {
                    sender.send(sample).expect("Send error");
                }
            },
            err_fn,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                for &sample in data.iter() {
                    let sample = (sample as i16) - i16::MAX;
                    sender.send(sample).expect("Send error");
                }
            },
            err_fn,
        )?,
    };

    stream.play()?;
    std::thread::sleep(Duration::from_secs(duration));

    drop(writer); // Ensure the writer is dropped and the file is flushed

    tokio::time::sleep(Duration::from_secs(duration)).await;

    println!("Recording complete. File saved as 'recording.wav'.");

    Ok(())
}

async fn audio_speech(transcribed_text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let _api_key = env::var("OPENAI_API_KEY")?;

    let request = CreateSpeechRequestArgs::default()
        .input(transcribed_text)
        .voice(Voice::Alloy)
        .model(SpeechModel::Tts1)
        .build()?;

    let response = client.audio().speech(request).await?;

    response.save("./audioo.mp3.").await?;

    // Supondo que `response.bytes` é o campo correto e contém os dados de áudio
    if !response.bytes.is_empty() {
        // Salva os dados de áudio em um arquivo
        std::fs::write("./audio.mp3", &response.bytes)?;
        println!("Audio saved to ./audio.mp3.");
    } else {
        println!("No audio received from the TTS service.");
    }

    Ok(())
}
