FROM rust:latest

WORKDIR /usr/src/whisper-job-manager
COPY whisper-job-manager whisper-job-manager
COPY whisper-job-manager-models whisper-job-manager-models
WORKDIR /usr/src/whisper-job-manager/whisper-job-manager

RUN apt-get update && apt-get install -y pipx
RUN pipx install openai-whisper

RUN cargo install --path .
EXPOSE 8080

CMD ["cargo", "run"]