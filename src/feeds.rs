// PARENT
#[poise::command(
    slash_command,
    subcommands("add", "remove")
]
pub async fn feed(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}