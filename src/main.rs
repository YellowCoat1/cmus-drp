use std::process::Command;
use discord_rich_presence::*;

#[derive(Debug)]
struct Song {
    path: String,
    playing: bool,
    title: Option<String>,
    artist: Option<String>,
    duration: i64, 
    position: i64,
}

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let mut client = DiscordIpcClient::new("1360519977819439134")?;
    let mut client_connected = false;
    let mut initial_timestamp: Option<i64> = None;

    loop {
        std::thread::sleep(std::time::Duration::from_secs_f64(1.0));

        let playing_status = cmus_status();
        
        if let Some(song) = playing_status {
            if !client_connected {
                let result = client.connect();
                if let Err(_) = result {
                    client_connected = false;
                    initial_timestamp = None;
                    continue;
                }
                client_connected = true;
                initial_timestamp = Some(get_time());
            }
            set_activity(&mut client, &song, &mut client_connected, initial_timestamp);
        } else {
            client_connected = false;
            client.close()?;
            initial_timestamp = None;
        }
    }
}

fn get_time() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}

fn set_activity(discord_client: &mut DiscordIpcClient, song: &Song, client_connected: &mut bool, initial_timestamp: Option<i64>) {
    let name = get_song_name(&song);
    let artist = get_song_artist(&song);

    let state = format!("{artist} ({})", if song.playing {get_timestamp(song)} else {"paused".to_string()} );

    let status_activity = activity::Activity::new()
        .details(name)
        .state(state.as_str())
        .timestamps(activity::Timestamps::new().start(initial_timestamp.unwrap()));


    
    let result = discord_client.set_activity(status_activity);
    if let Err(_) = result {
        *client_connected = false;
    }
}

fn get_timestamp(song: &Song) -> String {
    let duration = song.duration;
    let position = song.position;

    fn individual_timestamp(secs: i64) -> String {
        let minutes = secs/60;
        let seconds = secs % 60; 
        let formatted_seconds = if seconds < 10 { format!("0{}", seconds) } else { seconds.to_string() };
        format!("{}:{}", minutes, formatted_seconds)
    }

    format!("{} of {}", individual_timestamp(position), individual_timestamp(duration))
}

fn get_song_name<'a>(song: &'a Song) -> &'a str {
    if let Some(t) = &song.title {
        return t.as_str();
    } else {
        return song.path.as_str()
    }
}

fn get_song_artist<'a>(song: &'a Song) -> &'a str {
    if let Some(t) = &song.artist {
        return t.as_str();
    } else {
        return ""
    }
}



fn cmus_status() -> Option<Song> {
    let cmus_status_lines = get_cmus_raw_status()?; 
    lines_to_status(cmus_status_lines)
}

// returns formatted output of "cmus-remote -Q" which returns information about cmus.
fn get_cmus_raw_status() -> Option<Vec<Vec<String>>> {
    let output_raw = Command::new("cmus-remote")
        .args(["-Q"])
        .output();

    let output = match output_raw {
        Ok(v) => v,
        Err(_) => return None,
    };

    if !output.status.success() {
        return None
    }


    let output_string = String::from_utf8_lossy(&output.stdout);
    Some(output_string 
        .lines()
        .map(|line| line.split_whitespace().map(|word| word.to_string()).collect::<Vec<String>>())
        .collect())
}

fn join_past_index(vec: &Vec<String>) -> String {
    vec.get(2..)
        .unwrap_or(&[])
        .join(" ")
}

fn lines_to_status(lines: Vec<Vec<String>>) -> Option<Song> {
    let mut path: Option<String> = None;
    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;
    let mut duration: Option<i64> = None;
    let mut position: Option<i64> = None;
    let mut playing: bool = false;

    for line in lines {

        if line.len() == 0 {
            continue;
        }

        match line[0].as_str() {
            "status" => {
                playing = if line[1] == "playing" {true} else {false}
            }
            "file" => path = Some(line[1].clone()),
            "duration" => duration = Some(line[1].parse().unwrap()), 
            "position" => position = Some(line[1].parse().unwrap()),

            "tag" => match line[1].as_str() {
                "title" => title = Some(join_past_index(&line)),
                "artist" => artist = Some(join_past_index(&line)),
                _ => continue,
            }
            _ => continue,
        }    
    }

    if let None = path {
        return None;
    }

    Some(Song {
        path: path.unwrap(),
        playing,
        title,
        artist,
        duration: duration.expect("Couldn't find duration"),
        position: position.expect("Couldn't find position in song"),
    })
}
