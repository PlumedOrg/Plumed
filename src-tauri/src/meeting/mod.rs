//! Réunion (meeting) transcription, Plumed fork (written with Claude Code):
//! chunked transcription with word timestamps, live speaker diarization
//! (see `crate::diarization`) and the attribution of each word to a speaker.

pub mod backup;
pub mod session;
pub mod speakers;
pub mod state;
pub mod transcript;
pub mod voices;
pub mod words;
pub mod worker;
