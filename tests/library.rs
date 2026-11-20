//! `Library`: `add` replaces the samples of a name and `append` adds to
//! them, with one `/b_allocRead` per path and a `/b_query` of its buffer as
//! its completion, on the lowest free numbers of the range. `remove` and a
//! replacing `add` free the buffers of a name in their order. A call with
//! too few free numbers changes nothing. `reply` loads a sample at a
//! `/b_info` of its buffer, and fails it at a `/b_info` without data or at a
//! `/fail` of its `/b_allocRead`. `sample` truncates and wraps its index, and
//! `new` refuses an empty range and a range that starts under 0.

use std::error::Error;
use std::ops::Range;

use scsynth_dirt::{Library, LibraryError, Sample, SampleInfo};
use scsynth_osc::command::buffer::{BAllocRead, BFree, BQuery};
use scsynth_osc::reply::buffer::{BInfo, BufferInfo};
use scsynth_osc::reply::command_replies::{BufferCommand, Done, Fail};
use scsynth_osc::{BufferId, Command, Completion, Packet, RawArg, RawMessage, Reply, SyncId};

/// A library on the buffer numbers `buffers`.
fn library(buffers: Range<i32>) -> Library {
    Library::new(buffers.clone()).unwrap_or_else(|error| panic!("{buffers:?}: {error}"))
}

/// The buffer of the number `number`, 0 or more.
fn buffer(number: i32) -> BufferId {
    BufferId::new(number).unwrap_or_else(|| panic!("{number} is no buffer number"))
}

/// `/b_allocRead buffer path 0 -1`, with `/b_query buffer` as its completion
/// message.
fn alloc_read(number: i32, path: &str) -> Command {
    let query = Command::BQuery(BQuery {
        buffers: vec![buffer(number)],
    });
    Command::BAllocRead(BAllocRead {
        buffer: buffer(number),
        path: path.to_owned(),
        file_frame: 0,
        frames: -1,
        completion: Some(Completion::Message(Box::new(query))),
    })
}

/// `/b_free buffer`.
fn free(number: i32) -> Command {
    Command::BFree(BFree {
        buffer: buffer(number),
        completion: None,
    })
}

/// `/b_info` of one buffer.
fn b_info(number: i32, frames: i32, channels: i32, sample_rate: f32) -> Reply {
    Reply::BInfo(BInfo {
        buffers: vec![BufferInfo {
            buffer: buffer(number),
            frames,
            channels,
            sample_rate,
        }],
    })
}

/// `/fail command message value`, with the message of a file that scsynth
/// cannot open.
fn fail(command: &str, value: Option<i32>) -> Reply {
    Reply::Fail(Fail {
        command: command.to_owned(),
        message: "File 'a.wav' could not be opened: System error\n".to_owned(),
        value,
    })
}

/// The buffer number and the path of each sample of `name`.
fn content<'a>(library: &'a Library, name: &str) -> Vec<(i32, &'a str)> {
    library
        .samples(name)
        .unwrap_or_else(|| panic!("the library lacks {name}"))
        .iter()
        .map(|sample| (sample.buffer().get(), sample.path()))
        .collect()
}

/// Each name of `library`, with its samples.
fn snapshot(library: &Library) -> Vec<(String, Vec<Sample>)> {
    library
        .names()
        .map(|name| {
            let samples = library.samples(name).unwrap_or_default();
            (name.to_owned(), samples.to_vec())
        })
        .collect()
}

#[test]
fn add_returns_one_alloc_read_per_path_with_a_query_of_its_buffer_as_its_completion() {
    let mut library = library(10..20);
    assert_eq!(
        library.add("bd", ["bd/a.wav", "bd/b.wav"]),
        Ok(vec![alloc_read(10, "bd/a.wav"), alloc_read(11, "bd/b.wav")])
    );
    assert_eq!(
        content(&library, "bd"),
        [(10, "bd/a.wav"), (11, "bd/b.wav")]
    );
    for sample in library.samples("bd").unwrap_or_default() {
        assert_eq!(sample.info(), None, "{sample:?}");
        assert!(!sample.failed(), "{sample:?}");
    }
}

#[test]
fn the_bytes_of_an_alloc_read_carry_a_query_message_as_its_completion() {
    let mut library = library(10..20);
    let commands = library.add("bd", ["bd/a.wav"]).expect("a free number");
    let packet = Packet::Message(commands[0].clone());
    let bytes = packet.encode();
    let query = b"/b_query\0\0\0\0,i\0\0\0\0\0\x0a".to_vec();
    let raw = RawMessage {
        address: "/b_allocRead".to_owned(),
        args: vec![
            RawArg::Int(10),
            RawArg::String("bd/a.wav".to_owned()),
            RawArg::Int(0),
            RawArg::Int(-1),
            RawArg::Blob(query.clone()),
        ],
    };
    assert_eq!(
        Packet::<RawMessage>::decode(&bytes),
        Ok(Packet::Message(raw))
    );
    let message = Command::BQuery(BQuery {
        buffers: vec![buffer(10)],
    });
    assert_eq!(
        Packet::<Command>::decode(&query),
        Ok(Packet::Message(message))
    );
    assert_eq!(Packet::<Command>::decode(&bytes), Ok(packet));
}

#[test]
fn add_frees_the_buffers_that_it_replaces_in_their_order_and_takes_them_again() {
    let mut library = library(10..20);
    library
        .add("bd", ["a.wav", "b.wav", "c.wav"])
        .expect("free numbers");
    library.add("sn", ["s.wav"]).expect("a free number");
    assert_eq!(
        library.add("bd", ["d.wav", "e.wav"]),
        Ok(vec![
            free(10),
            free(11),
            free(12),
            alloc_read(10, "d.wav"),
            alloc_read(11, "e.wav"),
        ])
    );
    assert_eq!(content(&library, "bd"), [(10, "d.wav"), (11, "e.wav")]);
    assert_eq!(content(&library, "sn"), [(13, "s.wav")]);
    assert_eq!(
        library.add("bd", Vec::<&str>::new()),
        Ok(vec![free(10), free(11)])
    );
    assert_eq!(library.samples("bd"), Some(&[][..]));
}

#[test]
fn append_adds_samples_after_those_of_the_name_and_a_new_name_starts_empty() {
    let mut library = library(0..8);
    assert_eq!(
        library.append("bd", ["a.wav"]),
        Ok(vec![alloc_read(0, "a.wav")])
    );
    assert_eq!(
        library.append("bd", ["b.wav", "c.wav"]),
        Ok(vec![alloc_read(1, "b.wav"), alloc_read(2, "c.wav")])
    );
    assert_eq!(
        content(&library, "bd"),
        [(0, "a.wav"), (1, "b.wav"), (2, "c.wav")]
    );
    assert_eq!(library.append("empty", Vec::<&str>::new()), Ok(Vec::new()));
    assert_eq!(library.samples("empty"), Some(&[][..]));
    assert_eq!(library.names().collect::<Vec<_>>(), ["bd", "empty"]);
}

#[test]
fn remove_frees_the_buffers_of_a_name_in_their_order_and_takes_the_name_out() {
    let mut library = library(0..8);
    library.add("bd", ["a.wav", "b.wav"]).expect("free numbers");
    library.add("sn", ["s.wav"]).expect("a free number");
    assert_eq!(library.remove("bd"), [free(0), free(1)]);
    assert_eq!(library.samples("bd"), None);
    assert_eq!(library.names().collect::<Vec<_>>(), ["sn"]);
    assert_eq!(library.remove("bd"), []);
    assert_eq!(library.remove("missing"), []);
}

#[test]
fn a_sample_takes_the_lowest_free_number_of_the_range() {
    let mut library = library(10..20);
    library.add("a", ["a.wav"]).expect("a free number");
    library
        .add("b", ["b1.wav", "b2.wav"])
        .expect("free numbers");
    library.add("c", ["c.wav"]).expect("a free number");
    assert_eq!(library.remove("b"), [free(11), free(12)]);
    assert_eq!(
        library.append("d", ["d1.wav", "d2.wav", "d3.wav"]),
        Ok(vec![
            alloc_read(11, "d1.wav"),
            alloc_read(12, "d2.wav"),
            alloc_read(14, "d3.wav"),
        ])
    );
}

#[test]
fn a_call_with_too_few_free_numbers_changes_nothing_and_says_how_many_lack() {
    let mut library = library(0..3);
    library.add("bd", ["a.wav", "b.wav"]).expect("free numbers");
    library.reply(&b_info(0, 100, 1, 44_100.0));
    let before = snapshot(&library);
    assert_eq!(
        library.append("sn", ["x.wav", "y.wav"]),
        Err(LibraryError::TooFewBuffers(1))
    );
    assert_eq!(
        library.append("bd", ["c.wav", "d.wav", "e.wav"]),
        Err(LibraryError::TooFewBuffers(2))
    );
    assert_eq!(
        library.add("sn", ["x.wav", "y.wav"]),
        Err(LibraryError::TooFewBuffers(1))
    );
    assert_eq!(
        library.add("bd", ["c.wav", "d.wav", "e.wav", "f.wav"]),
        Err(LibraryError::TooFewBuffers(1))
    );
    assert_eq!(snapshot(&library), before);
    assert_eq!(
        library.add("bd", ["c.wav", "d.wav", "e.wav"]),
        Ok(vec![
            free(0),
            free(1),
            alloc_read(0, "c.wav"),
            alloc_read(1, "d.wav"),
            alloc_read(2, "e.wav"),
        ])
    );
}

#[test]
fn a_b_info_loads_the_samples_of_its_buffers() {
    let mut library = library(10..20);
    library.add("bd", ["a.wav", "b.wav"]).expect("free numbers");
    library.reply(&Reply::BInfo(BInfo {
        buffers: vec![
            BufferInfo {
                buffer: buffer(11),
                frames: 22_050,
                channels: 2,
                sample_rate: 44_100.0,
            },
            BufferInfo {
                buffer: buffer(10),
                frames: 48_000,
                channels: 1,
                sample_rate: 48_000.0,
            },
        ],
    }));
    let samples = library.samples("bd").unwrap_or_default();
    let info = |frames, channels, sample_rate| SampleInfo {
        frames,
        channels,
        sample_rate,
    };
    assert_eq!(samples[0].info(), Some(info(48_000, 1, 48_000.0)));
    assert_eq!(samples[1].info(), Some(info(22_050, 2, 44_100.0)));
    assert!(samples.iter().all(|sample| !sample.failed()), "{samples:?}");
}

#[test]
fn a_b_info_without_data_fails_its_sample() {
    for (frames, channels, sample_rate) in [
        (0, 0, 0.0),
        (0, 1, 44_100.0),
        (-1, 1, 44_100.0),
        (100, 0, 44_100.0),
        (100, -1, 44_100.0),
        (100, 1, 0.0),
        (100, 1, -44_100.0),
        (100, 1, f32::NAN),
        (100, 1, f32::INFINITY),
    ] {
        let mut library = library(0..4);
        library.add("bd", ["a.wav"]).expect("a free number");
        library.reply(&b_info(0, frames, channels, sample_rate));
        let sample = &library.samples("bd").unwrap_or_default()[0];
        assert!(sample.failed(), "{frames} {channels} {sample_rate}");
        assert_eq!(sample.info(), None, "{frames} {channels} {sample_rate}");
    }
}

#[test]
fn a_fail_of_the_alloc_read_of_a_buffer_fails_its_sample() {
    let mut library = library(10..20);
    library.add("bd", ["a.wav", "b.wav"]).expect("free numbers");
    library.reply(&fail("/b_allocRead", Some(11)));
    let samples = library.samples("bd").unwrap_or_default();
    assert!(!samples[0].failed());
    assert!(samples[1].failed());
    assert_eq!(samples[1].info(), None);
}

#[test]
fn a_later_reply_replaces_what_an_earlier_reply_said() {
    let mut library = library(0..4);
    library.add("bd", ["a.wav"]).expect("a free number");
    let loaded = Some(SampleInfo {
        frames: 100,
        channels: 1,
        sample_rate: 44_100.0,
    });
    library.reply(&fail("/b_allocRead", Some(0)));
    library.reply(&b_info(0, 100, 1, 44_100.0));
    assert_eq!(library.samples("bd").unwrap_or_default()[0].info(), loaded);
    library.reply(&b_info(0, 0, 0, 0.0));
    assert!(library.samples("bd").unwrap_or_default()[0].failed());
    library.reply(&b_info(0, 100, 1, 44_100.0));
    library.reply(&fail("/b_allocRead", Some(0)));
    assert!(library.samples("bd").unwrap_or_default()[0].failed());
}

#[test]
fn a_fail_of_another_command_or_of_no_buffer_of_the_library_does_nothing() {
    let mut library = library(10..20);
    library.add("bd", ["a.wav"]).expect("a free number");
    let before = snapshot(&library);
    for reply in [
        fail("/b_read", Some(10)),
        fail("/b_allocReadChannel", Some(10)),
        fail("/b_allocRead", Some(11)),
        fail("/b_allocRead", Some(30)),
        fail("/b_allocRead", Some(-1)),
        fail("/b_allocRead", None),
    ] {
        library.reply(&reply);
        assert_eq!(snapshot(&library), before, "{reply:?}");
    }
}

#[test]
fn a_reply_for_a_buffer_outside_the_library_does_nothing() {
    let mut library = library(10..20);
    library.add("bd", ["a.wav"]).expect("a free number");
    let before = snapshot(&library);
    for number in [11, 9, 20] {
        library.reply(&b_info(number, 0, 0, 0.0));
        library.reply(&b_info(number, 100, 1, 44_100.0));
        assert_eq!(snapshot(&library), before, "buffer {number}");
    }
}

#[test]
fn every_other_reply_does_nothing() {
    let mut library = library(10..20);
    library.add("bd", ["a.wav"]).expect("a free number");
    let before = snapshot(&library);
    for reply in [
        Reply::Done(Done::Buffer {
            command: BufferCommand::BAllocRead,
            buffer: buffer(10),
        }),
        Reply::Synced(SyncId(10)),
    ] {
        library.reply(&reply);
        assert_eq!(snapshot(&library), before, "{reply:?}");
    }
}

#[test]
fn sample_wraps_the_index_truncated_toward_0_to_the_count() {
    let mut library = library(0..8);
    library
        .add("bd", ["0.wav", "1.wav", "2.wav"])
        .expect("free numbers");
    for (n, path) in [
        (0.0, "0.wav"),
        (2.0, "2.wav"),
        (3.0, "0.wav"),
        (7.0, "1.wav"),
        (-1.0, "2.wav"),
        (-3.0, "0.wav"),
        (-4.0, "2.wav"),
        (1.9, "1.wav"),
        (-0.5, "0.wav"),
        (-1.5, "2.wav"),
        (f64::NAN, "0.wav"),
        (3e9, "1.wav"),
        (f64::INFINITY, "1.wav"),
    ] {
        let sample = library.sample("bd", n).unwrap_or_else(|| panic!("bd:{n}"));
        assert_eq!(sample.path(), path, "bd:{n}");
    }
    assert_eq!(library.sample("missing", 0.0), None);
    library
        .append("empty", Vec::<&str>::new())
        .expect("no number");
    assert_eq!(library.sample("empty", 0.0), None);
}

#[test]
fn the_names_come_in_the_order_of_their_bytes() {
    let mut library = library(0..16);
    for name in ["sn", "bd2", "808", "bd", "_x", "Kick"] {
        library.add(name, ["a.wav"]).expect("a free number");
    }
    assert_eq!(
        library.names().collect::<Vec<_>>(),
        ["808", "Kick", "_x", "bd", "bd2", "sn"]
    );
}

#[test]
#[allow(
    clippy::reversed_empty_ranges,
    reason = "new must refuse a reversed range as empty"
)]
fn new_refuses_an_empty_range_and_a_range_that_starts_under_0() {
    for (buffers, error) in [
        (5..5, LibraryError::EmptyRange(5..5)),
        (5..3, LibraryError::EmptyRange(5..3)),
        (-3..-3, LibraryError::EmptyRange(-3..-3)),
        (-1..4, LibraryError::NegativeStart(-1..4)),
        (i32::MIN..0, LibraryError::NegativeStart(i32::MIN..0)),
    ] {
        assert_eq!(
            Library::new(buffers.clone()).err(),
            Some(error),
            "{buffers:?}"
        );
    }
    let mut library = library(i32::MAX - 1..i32::MAX);
    assert_eq!(
        library.add("bd", ["a.wav"]),
        Ok(vec![alloc_read(i32::MAX - 1, "a.wav")])
    );
    assert_eq!(
        library.append("bd", ["b.wav"]),
        Err(LibraryError::TooFewBuffers(1))
    );
}

#[test]
fn an_error_says_what_is_wrong() {
    for (error, message) in [
        (
            LibraryError::EmptyRange(5..5),
            "the range 5..5 of buffer numbers is empty",
        ),
        (
            LibraryError::NegativeStart(-1..4),
            "the range -1..4 of buffer numbers starts under 0, but a buffer number is 0 or more",
        ),
        (
            LibraryError::TooFewBuffers(3),
            "the range of buffer numbers is 3 short of free numbers for the new samples",
        ),
    ] {
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none(), "{error}");
    }
}
