//! Dev tool: polish stdin through any provider, exactly as the app will.
use retone_core::{models, prompt::{self, Length, Tone}, providers::{registry::{build_provider, ProviderId}, CompletionRequest},
                  settings::{KeyringStore, SecretStore, Settings}};
use std::io::Read;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(id) = args.first().and_then(|k| ProviderId::from_key(k)) else {
        eprintln!("usage: retone-dev <{}> [--model M] [--tone T] [--length shorter|same|longer] [--context TEXT] [--set-key KEY]",
                  ProviderId::ALL.map(|p| p.key()).join("|"));
        std::process::exit(2);
    };
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let secrets = KeyringStore;
    if let Some(k) = flag("--set-key") { secrets.set(id.key(), &k).expect("keychain write"); eprintln!("key saved"); return; }

    let mut draft = String::new();
    std::io::stdin().read_to_string(&mut draft).expect("read stdin");
    let tone = flag("--tone").and_then(|t| Tone::from_name(&t)).unwrap_or(Tone::Standard);
    let length = match flag("--length").as_deref() { Some("shorter") => Length::Shorter, Some("longer") => Length::Longer, _ => Length::Same };
    let context = flag("--context").unwrap_or_default();
    let s = Settings::default();
    let p = prompt::build(&prompt::PromptInput { style: &s.style, tone, length, context: &context, draft: &draft });
    let model = flag("--model").or_else(|| models::default_model(id.key()));
    let started = std::time::Instant::now();
    match build_provider(id, &s, &secrets).complete(&CompletionRequest { system: p.system, user: p.user, model }).await {
        Ok(text) => { println!("{text}"); eprintln!("[{} in {:.1}s]", id.label(), started.elapsed().as_secs_f32()); }
        Err(e) => { eprintln!("error: {e}"); std::process::exit(1); }
    }
}
