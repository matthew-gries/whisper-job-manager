# OpenAI Whisper Transcription Job Manager

This is a tool to help manage instances of OpenAI Whisper running transcription jobs on audio or video files. The user can submit requests to transcribe audio or video files, and the server will queue requests and manage active instances of OpenAI Whisper. The user can query the status of jobs, cancel any queued or active jobs, and download the transcripts from completed jobs.

# Basic Usage

In the `whisper-job-manager` package:  
* Fill out the `config.json` file:
  * `videoStoragePath`: the folder containing audio and video files to transcribe
  * `host`: the hostname for the connection
  * `port`: the port of the connection
* Run the `cargo run` command

In another shell, in the `whisper-job-manager-cli` package, run the following command:

`cargo run -- -e <HOST>:<PORT> <FILEPATH>`

Where:
* HOST - the hostname of the server
* PORT - the port of the server
* FILEPATH - the path of the file to transcribe, expected to be the relative path of the file in `videoStoragePath`

Run `cargo run -- -h` for more options..