// The golden files of tests/fixtures: the core synth definitions of SuperDirt
// for 1, 2 and 4 channels, as sclang 3.14.1 builds and writes them with no
// server. Run it from the root of the repository, with absolute paths, since
// ~/.bin/sclang changes the directory. Without the class SwitchDelay, it
// writes definitions/core_<n>.scsyndef, the 25 definitions of n channels in
// the order of their names. Then it loads core-synths.scd again with
// maxSampleNumChannels of SuperDirt at 4, and writes
// definitions/players_3_4_<n>.scsyndef, the 6 sample players of 3 and 4
// sample channels in the order of their names. Last, it writes vowels.json,
// the formants of the Vowel quark:
//
//   sclang -l "$PWD/generator/sclang_conf.yaml" "$PWD/generator/fixtures.scd"
//
// With the class SwitchDelay of sc3-plugins, it writes
// definitions/switch_delay_<n>.scsyndef, the definition dirt_delay<n> alone:
//
//   sclang -l "$PWD/generator/sc3_plugins/sclang_conf.yaml" \
//     "$PWD/generator/fixtures.scd"
//
// It exits with 1 if sclang is not 3.14.1, if the clone of SuperDirt or of
// Vowel is missing, is not at its commit or has changes to tracked files, if
// a class or a method comes from outside the include paths of the
// configuration, or if the core files of SuperDirt do not add 25 definitions
// for a channel count. It also exits with 1 if the second load does not add
// exactly the 6 players.
(
var root = thisProcess.nowExecutingPath.dirname.dirname;
var fixtures = root +/+ "tests" +/+ "fixtures";
var definitions = fixtures +/+ "definitions";
var includes = LanguageConfig.includePaths.collect(_.withTrailingSlash);
// A class name that sclang lacks stops the compile of the whole script, which
// then never exits (measured), so the classes of the extensions go by symbol.
var dirtClass = \SuperDirt.asClass;
var libraryClass = \DirtSoundLibrary.asClass;
var vowelClass = \Vowel.asClass;
var switchDelay = \SwitchDelay.asClass.notNil;
// sclang 3.14.1 runs the code after 1.exit (measured), so the throw stops it.
var fail = { |message|
	("fixtures.scd: " ++ message).postln;
	1.exit;
	Error(message).throw;
};
var known = { |item|
	var path = item.filenameSymbol.asString;
	includes.any { |include| path.beginsWith(include) }
};
var checkCommit = { |class, commit|
	var folder = class.filenameSymbol.asString.dirname;
	var git = { |command|
		"git --no-optional-locks -C % %".format(folder.shellQuote, command).unixCmdGetStdOut
	};
	var head = git.("rev-parse HEAD").stripWhiteSpace;
	if (head != commit) {
		fail.("the clone of % in % is at %, not %".format(class, folder, head, commit));
	};
	if (git.("status --porcelain --untracked-files=no").notEmpty) {
		fail.("the clone of % in % has changes to tracked files".format(class, folder));
	};
};
var added = { |before|
	SynthDescLib.global.synthDescs.keys.asArray.reject { |name| before.includes(name) }.sort
};
var write, writeVowels, server, synths;

try {
	if (Main.version != "3.14.1") {
		fail.("sclang % is not 3.14.1".format(Main.version));
	};
	if (dirtClass.isNil or: { vowelClass.isNil }) {
		fail.("the configuration lacks SuperDirt or the Vowel quark");
	};
	checkCommit.(dirtClass, "a3046d1e693eb1c7f5502fb1d0a27d59099737f8");
	checkCommit.(vowelClass, "ab59caa870201ecf2604b3efdd2196e21a8b5446");
	Class.allClasses.do { |class|
		if (known.(class).not) {
			fail.("% comes from %".format(class, class.filenameSymbol));
		};
		(class.methods ? []).do { |method|
			if (known.(method).not) {
				fail.("% comes from %".format(method, method.filenameSymbol));
			};
		};
	};

	// Server.default is the scsynth on UDP 57110 unless set, and SuperDirt
	// takes it where it gets no server.
	server = Server(\scsynth_dirt_fixtures, NetAddr("127.0.0.1", 57999));
	Server.default = server;

	write = { |name, defs|
		var stream = CollStream.on(Int8Array.new(256));
		defs.writeDef(stream);
		File.use(definitions +/+ name ++ ".scsyndef", "wb", { |file|
			file.putAll(stream.collection);
		});
		"fixtures.scd: wrote definitions/%.scsyndef (definitions: %, bytes: %)"
			.format(name, defs.size, stream.collection.size).postln;
	};

	writeVowels = {
		var registers = [\soprano, \alto, \tenor, \bass, \counterTenor];
		var vowels = [\a, \e, \i, \o, \u];
		// 17 significant digits keep every bit of a Float.
		var numbers = { |values|
			"[" ++ values.collect { |value| value.asFloat.asStringPrec(17) }.join(", ") ++ "]"
		};
		var text = "{\n" ++ registers.collect { |register|
			"  \"%\": {\n".format(register) ++ vowels.collect { |name|
				var vowel = vowelClass.new(name, register);
				"    \"%\": {\"freqs\": %, \"amps\": %, \"rqs\": %}".format(name,
					numbers.(vowel.freqs), numbers.(vowel.amps), numbers.(vowel.rqs))
			}.join(",\n") ++ "\n  }"
		}.join(",\n") ++ "\n}\n";
		File.use(fixtures +/+ "vowels.json", "w", { |file| file.write(text) });
		"fixtures.scd: wrote vowels.json (vowels: %, bytes: %)"
			.format(registers.size * vowels.size, text.size).postln;
	};

	synths = dirtClass.filenameSymbol.asString.dirname.dirname +/+ "synths";
	definitions.mkdir;
	[1, 2, 4].do { |channels|
		var dirt = dirtClass.newCopyArgs(channels, server);
		var environment = (dirt: dirt);
		var before = SynthDescLib.global.synthDescs.keys;
		var names, players;
		dirt.modules = [];
		dirt.soundLibrary = libraryClass.new(server, channels);
		// The extras of SuperDirt throw for 1 channel: compressor sends mean to
		// one channel.
		["core-modules.scd", "core-synths-global.scd", "core-synths.scd"].do { |file|
			environment.use { (synths +/+ file).load };
		};
		names = added.(before);
		if (names.size != 25) {
			fail.("the core files add % definitions for numChannels %, not 25"
				.format(names.size, channels));
		};
		if (switchDelay) {
			write.("switch_delay_" ++ channels,
				[SynthDescLib.global.at(("dirt_delay" ++ channels).asSymbol).def]);
		} {
			write.("core_" ++ channels, names.collect { |name| SynthDescLib.global.at(name).def });
			before = SynthDescLib.global.synthDescs.keys;
			dirtClass.maxSampleNumChannels = 4;
			environment.use { (synths +/+ "core-synths.scd").load };
			dirtClass.maxSampleNumChannels = 2;
			names = added.(before);
			players = ["sample_3_", "sample_4_", "sample_long_3_", "sample_long_4_",
				"stretchsample_3_", "stretchsample_4_"]
				.collect { |stem| ("dirt_" ++ stem ++ channels).asSymbol };
			if (names != players) {
				fail.("core-synths.scd adds % for maxSampleNumChannels 4 and numChannels %, not %"
					.format(names, channels, players));
			};
			write.("players_3_4_" ++ channels, names.collect { |name| SynthDescLib.global.at(name).def });
		};
	};
	if (switchDelay.not) { writeVowels.() };
	0.exit;
} { |error|
	error.reportError;
	1.exit;
};
)
