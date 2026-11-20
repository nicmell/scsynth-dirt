// The golden events of tests/fixtures: the bundles that SuperDirt sends for
// the sounds of the scenarios below, as sclang 3.14.1 builds them
// with no server. Run it from the root of the repository, with absolute
// paths, since ~/.bin/sclang changes the directory. It writes events.json:
//
//   sclang -l "$PWD/generator/sclang_conf.yaml" "$PWD/generator/events.scd"
//
// Each scenario runs on a new instance of SuperDirt with 2 orbits, whose
// buses sit where the crate puts them, and with a library of buffers that
// scsynth never reads. An event of the script takes the place of the
// address of the server, and keeps each bundle apart, with its time. The
// shape of events.json:
//
//   {"scenarios": [scenario, ...]}, in the order of the scenarios below
//   scenario: {"name", "config": {"channels", "orbits", "first_bus",
//     "out_bus", "sample_rate"}, "library": [{"name", "samples": [{"buffer",
//     "frames", "channels", "sample_rate"}, ...]}, ...] in the order of the
//     adds, "start": [bundle, ...] of initNodeTree, one per orbit in order,
//     "steps": [step, ...]}
//   step: {"set": {"orbit", "name", "value"}}, an orbit default, or
//     {"sound": {"when", "parameters": {name: value, ...}}, "bundles":
//     [bundle, ...]}, with no bundle for a sound that SuperDirt drops
//   bundle: {"time": seconds or null, "messages": [message, ...]}
//   message: [address, argument, ...]
//
// An address starts with a slash. An Integer has no point and no exponent,
// and a Float has a point or an exponent, with a precision of 17 significant
// digits. A Symbol or a String is a string, nil is null, an array argument
// is a list, and cutAll is true. The gate lacks its argument sample, a hash
// of sclang that changes at each run.
//
// It exits with 1 if a check of common.scd fails, if SuperDirt throws on a
// sound, drops a sound that the scenario plays or plays one that it drops,
// or if it sends what the script does not expect.
(
var root = thisProcess.nowExecutingPath.dirname.dirname;
var fixtures = root +/+ "tests" +/+ "fixtures";
var common = (thisProcess.nowExecutingPath.dirname +/+ "common.scd").load;
// The classes go by symbol, as common.scd says.
var dirtClass = \SuperDirt.asClass;
var libraryClass = \DirtSoundLibrary.asClass;
var orbitClass = \DirtOrbit.asClass;
var eventClass = \DirtEvent.asClass;
var sampleRate = 48000.0;
var orbitCount = 2;
// The first bus and the first buffer differ from those of a default server,
// 4 (measured) and 0.
var firstBus = 16;
var outBus = 0;
var firstBuffer = 100;
var library = [
	[\bd, [[22050, 1, 44100.0], [11025, 1, 44100.0]]],
	[\sn, [[48000, 2, 48000.0]]],
	// SuperDirt plays a buffer of more than 16777216 frames with the long player.
	[\long, [[16777217, 1, 44100.0]]],
	[\x_bd, [[24000, 1, 32000.0]]]
];
var modules = [\sound, \vowel, \shape, \hpf, \bpf, \crush, \coarse, \lpf, \pshift,
	\envelope, \grenvelo, \tremolo, \phaser];
var scenarios = [
	[\start, 2, []],
	[\names, 2, [
		[\play, 1.0, [s: \bd]],
		[\play, 2.0, [s: \bd, n: 1.0]],
		[\play, 3.0, [sound: 'bd:1']],
		[\play, 4.0, [sound: 'bd:x']],
		[\play, 5.0, [sound: 'bd:1x']],
		[\play, 6.0, [sound: \bd, n: 1.0]],
		[\play, 7.0, [sound: 'bd:1', n: 0.0]],
		[\play, 8.0, [s: \bd, n: 2.0]],
		[\play, 9.0, [s: \bd, n: -1.0]],
		[\play, 10.0, [s: \bd, n: 1.7]],
		[\play, 11.0, [s: \bd, n: -0.5]],
		[\play, 12.0, [s: \sn]],
		[\play, 13.0, [s: \long]],
		[\play, 14.0, [s: \bd, bank: \x]],
		[\drop, 15.0, [s: '']]
	]],
	[\time, 2, [
		[\play, 1.1, [s: \bd, lag: 0.1]],
		[\play, 2.2, [s: \bd, offset: 0.2, speed: 2.0]],
		[\play, 3.3, [s: \bd, lag: 0.05, offset: 0.1, speed: 1.5]],
		[\play, 4.4, [s: \bd, offset: 0.1, speed: -1.5]],
		[\play, 5.5, [s: \bd, offset: 0.25, unit: \c, cps: 0.5]],
		[\play, 6.6, [s: \bd, offset: 0.25, speed: 2.0, accelerate: 1.0]],
		[\set, 0, \lag, 0.02],
		[\play, 7.7, [s: \bd]]
	]],
	[\span, 2, [
		[\play, 1.0, [s: \bd, speed: -1.0]],
		[\play, 2.0, [s: \bd, speed: -2]],
		[\play, 3.0, [s: \bd, accelerate: 1.0]],
		[\play, 4.0, [s: \bd, speed: -1.0, accelerate: -3.0]],
		[\play, 5.0, [s: \bd, begin: 0.25, end: 0.75]],
		[\play, 6.0, [s: \bd, begin: 0.5]],
		[\play, 7.0, [s: \bd, end: 0.5]],
		[\play, 8.0, [s: \bd, begin: 0.75, end: 0.25]],
		[\play, 9.0, [s: \bd, unit: \c, cps: 0.5]],
		[\play, 10.0, [s: \bd, unit: \c, speed: 2.0, cps: 0.75]],
		[\play, 11.0, [s: \bd, unit: \s, speed: 2.0]],
		[\play, 12.0, [s: \bd, unit: \s, begin: 0.25, end: 0.5]],
		[\play, 13.0, [s: \bd, unit: \rate, speed: 2.0]],
		[\play, 14.0, [s: \bd, loop: 2.0]],
		[\play, 15.0, [s: \bd, loop: -0.5]],
		[\play, 16.0, [s: \bd, legato: 0.5]],
		[\play, 17.0, [s: \bd, legato: 0.5, delta: 0.25]],
		[\play, 18.0, [s: \bd, legato: 1.0, dur: 0.25]],
		[\play, 19.0, [s: \bd, legato: 2.0]],
		[\play, 20.0, [s: \bd, sustain: 0.2]],
		[\play, 21.0, [s: \bd, sustain: 2.0]],
		[\play, 22.0, [s: \bd, sustain: 0.004]],
		[\drop, 23.0, [s: \bd, sustain: 0.0001]],
		[\play, 24.0, [s: \bd, timescale: 2.0]],
		[\play, 25.0, [s: \bd, timescale: 0.5, timescalewin: 0.25]],
		[\play, 26.0, [s: \bd, fadeTime: 0.01]],
		[\play, 27.0, [s: \sn, speed: 0.5]]
	]],
	[\pitch, 2, [
		[\play, 1.0, [s: \bd, note: 12.0]],
		[\play, 2.0, [s: \bd, octave: 4]],
		[\play, 3.0, [s: \bd, note: 3.0, octave: 6]],
		[\play, 4.0, [s: \bd, freq: 440.0]],
		[\play, 5.0, [s: \bd, midinote: 67.0]],
		[\play, 6.0, [s: \bd, freq: 300.0, midinote: 50.0]],
		[\play, 7.0, [s: \bd, midinote: 50.0, note: 7.0]],
		[\play, 8.0, [s: \bd, freq: 523.25, unit: \c, cps: 0.5]],
		[\play, 9.0, [s: \sn, note: -12.0]]
	]],
	[\level_and_place, 2, [
		[\play, 1.0, [s: \bd, gain: 1.2]],
		[\play, 2.0, [s: \bd, overgain: 0.5]],
		[\play, 3.0, [s: \bd, amp: 0.8]],
		[\set, 0, \amp, 0.6],
		[\play, 4.0, [s: \bd]],
		[\play, 5.0, [s: \bd, amp: 0.3]],
		[\play, 6.0, [s: \bd, orbit: 1]],
		[\play, 7.0, [s: \bd, pan: 0.0]],
		[\play, 8.0, [s: \bd, pan: 1.0]],
		[\play, 9.0, [s: \bd, channel: 1]],
		[\play, 10.0, [s: \bd, pan: 0.25, channel: 1]]
	]],
	[\modules, 2, [
		[\play, 1.0, [s: \bd, vowel: \a]],
		[\play, 2.0, [s: \bd, vowel: \e, resonance: 0.4]],
		[\play, 3.0, [s: \bd, vowel: \i]],
		[\play, 4.0, [s: \bd, vowel: \o]],
		[\play, 5.0, [s: \bd, vowel: \u]],
		[\play, 6.0, [s: \bd, vowel: \x]],
		[\play, 7.0, [s: \bd, shape: 0.5]],
		[\play, 8.0, [s: \bd, hcutoff: 1000.0]],
		[\play, 9.0, [s: \bd, hcutoff: 1000.0, hresonance: 0.3]],
		[\play, 10.0, [s: \bd, bandf: 1200.0]],
		[\play, 11.0, [s: \bd, bandf: 1200.0, bandq: 5.0]],
		[\play, 12.0, [s: \bd, crush: 4.0]],
		[\play, 13.0, [s: \bd, coarse: 1]],
		[\play, 14.0, [s: \bd, coarse: 4]],
		[\play, 15.0, [s: \bd, cutoff: 800]],
		[\play, 16.0, [s: \bd, cutoff: 800.0, resonance: 0.3]],
		[\play, 17.0, [s: \bd, psrate: 2.0]],
		[\play, 18.0, [s: \bd, psrate: 2.0, psdisp: 0.1]],
		[\play, 19.0, [s: \bd, attack: 0.01]],
		[\play, 20.0, [s: \bd, release: 0.2]],
		[\play, 21.0, [s: \bd, attack: 0.01, hold: 0.1, release: 0.2, curve: -4.0]],
		[\play, 22.0, [s: \bd, tilt: 0.5]],
		[\play, 23.0, [s: \bd, tilt: 0.5, plat: 0.2, curve: 2.0]],
		[\play, 24.0, [s: \bd, tremolorate: 4.0]],
		[\play, 25.0, [s: \bd, tremolorate: 4.0, tremolodepth: 0.5]],
		[\play, 26.0, [s: \bd, phaserrate: 2.0]],
		[\play, 27.0, [s: \bd, phaserrate: 2.0, phaserdepth: 0.5]],
		[\play, 28.0, [s: \bd, vowel: \o, resonance: 0.2, shape: 0.3, hcutoff: 200.0,
			hresonance: 0.1, bandf: 1500.0, bandq: 3.0, crush: 6.0, coarse: 3, cutoff: 3000.0,
			psrate: 1.5, psdisp: 0.05, attack: 0.02, hold: 0.1, release: 0.3, curve: -2.0,
			tilt: 0.4, plat: 0.3, tremolorate: 3.0, tremolodepth: 0.4, phaserrate: 1.0,
			phaserdepth: 0.6]]
	]],
	[\global_synths, 2, [
		[\play, 1.0, [s: \bd, delay: 0.5, delaytime: 0.25, delayfeedback: 0.6, delaySend: 0.8]],
		[\play, 2.0, [s: \bd, delay: 0.5, delaytime: 0.25, delayfeedback: 0.6, delaySend: 0.8]],
		[\play, 3.0, [s: \bd]],
		[\play, 4.0, [s: \bd, room: 0.3, size: 0.8]],
		[\play, 5.0, [s: \bd]],
		[\play, 6.0, [s: \bd, dry: 0.5]],
		[\play, 7.0, [s: \bd, lock: 1, cps: 0.5]],
		[\play, 8.0, [s: \bd, leslie: 0.5, lrate: 6.7, lsize: 0.3]],
		[\play, 9.0, [s: \bd]],
		[\play, 10.0, [s: \bd, limitertype: 2]],
		[\play, 11.0, [s: \bd]],
		[\play, 12.0, [s: \bd, rmsReplyRate: 20, rmsPeakLag: 2]],
		[\set, 0, \size, 0.5],
		[\play, 13.0, [s: \bd]]
	]],
	// Each sound starts before the end of every earlier one, since no /n_end
	// takes a sound out of the flotsam of SuperDirt.
	[\cut, 2, [
		[\play, 1.0, [s: \bd, cut: 1]],
		[\play, 1.05, [s: \bd, cut: 1]],
		[\play, 1.1, [s: \bd, cut: 1]],
		[\play, 1.1, [s: \bd, cut: 1]],
		[\play, 1.15, [s: \bd]],
		[\play, 1.2, [s: \bd, cut: -1]],
		[\play, 1.25, [s: \sn, cut: -1]],
		[\play, 1.3, [s: \bd, n: 1.0, cut: -1]],
		[\play, 1.35, [s: \bd, cut: 1]],
		[\play, 1.4, [s: \bd, cut: 2]],
		[\play, 1.45, [s: \bd, orbit: 1, cut: 1]],
		[\play, 1.45, [s: \bd, orbit: 1, cut: 1, cutAll: true]]
	]],
	[\orbits, 2, [
		[\play, 1.0, [s: \bd, orbit: 1]],
		[\play, 2.0, [s: \bd, orbit: 2]],
		[\play, 3.0, [s: \bd, orbit: -1]],
		[\play, 4.0, [s: \bd, orbit: 1.7]],
		[\play, 5.0, [s: \bd, orbit: -0.5]]
	]]
] ++ [1, 4].collect { |channels|
	[("channels_" ++ channels).asSymbol, channels, [
		[\play, 1.0, [s: \bd]],
		[\play, 2.0, [s: \sn, pan: 0.25, channel: 1]],
		[\play, 3.0, [s: \bd, orbit: 1, vowel: \a, room: 0.3]]
	]]
};
var fail, server, recorder, synths, argument, run, texts, text;
var take = {
	var bundles = recorder[\bundles];
	recorder[\bundles] = List.new;
	bundles
};
var list = { |items, indent|
	if (items.isEmpty) { "[]" } { "[\n" ++ items.join(",\n") ++ "\n" ++ indent ++ "]" }
};
var number = { |item|
	var digits;
	case
	{ item.isKindOf(Integer) } { item.asString }
	{ item.isKindOf(Float) and: { item.isNaN.not } and: { item.abs != inf } } {
		// 17 significant digits keep every bit of a Float, and %g writes a whole
		// number with no point.
		digits = item.asStringPrec(17);
		if (digits.includes($.) or: { digits.includes($e) }) { digits } { digits ++ ".0" }
	}
	{ fail.("cannot write % as a number".format(item.asCompileString)) }
};
var string = { |item|
	var chars = item.asString;
	if (chars.any { |char| char.ascii < 32 or: { char.ascii > 126 } or: { char == $" } or: { char == $\\ } }) {
		fail.("cannot write % as a string".format(chars.asCompileString));
	};
	"\"" ++ chars ++ "\""
};
var parameter = { |item|
	case
	{ item === true } { "true" }
	{ item.isKindOf(Integer) or: { item.isKindOf(Float) } } { number.(item) }
	{ item.isKindOf(Symbol) } { string.(item) }
	{ fail.("cannot write the parameter %".format(item.asCompileString)) }
};
// The numbers are the commands of scsynth, as Node.sc sends them.
var address = { |name|
	var path = case
	{ name == 9 } { "/s_new" }
	{ name == 12 } { "/n_run" }
	{ name == 15 } { "/n_set" }
	{ name == 21 } { "/g_new" }
	{ name === \s_new } { "/s_new" }
	{ name === \g_new } { "/g_new" }
	{ name.isKindOf(String) or: { name.isKindOf(Symbol) } } { name.asString }
	{ "" };
	if (path.beginsWith("/").not) { fail.("cannot name the address %".format(name.asCompileString)) };
	path
};
// sclang marks an array argument with $[ and $] in the flat message.
var nest = { |items|
	var stack = [List.new];
	items.do { |item|
		case
		{ item == $[ } { stack = stack.add(List.new) }
		{ item == $] and: { stack.size > 1 } } { stack[stack.size - 2].add(stack.pop) }
		{ item == $] } { fail.("an unmatched $] in %".format(items.asCompileString)) }
		{ stack.last.add(item) }
	};
	if (stack.size != 1) { fail.("an unmatched $[ in %".format(items.asCompileString)) };
	stack[0]
};
var message = { |item, gate|
	var path, rest, index;
	if (item.isKindOf(Array).not or: { item.isEmpty }) {
		fail.("cannot write % as a message".format(item.asCompileString));
	};
	path = address.(item[0]);
	rest = item.drop(1);
	if (path == "/s_new" and: { rest[0].asString == gate }) {
		index = rest.indexOf(\sample);
		if (index.isNil or: { index < 4 } or: { index.odd }) {
			fail.("the gate lacks its argument sample: %".format(item.asCompileString));
		};
		2.do { rest.removeAt(index) };
	};
	"[" ++ ([string.(path)] ++ nest.(rest).collect(argument)).join(", ") ++ "]"
};
var bundle = { |item, gate, indent|
	var time = item[0], messages = item[1];
	if (messages.isEmpty) { fail.("an empty bundle at %".format(time)) };
	"%{\"time\": %, \"messages\": %}".format(indent,
		if (time.isNil) { "null" } { number.(time) },
		list.(messages.collect { |each| indent ++ "  " ++ message.(each, gate) }, indent))
};
var player = { |item|
	var name = item[1].asString;
	item[0] === \s_new and: {
		name.beginsWith("dirt_sample_") or: { name.beginsWith("dirt_stretchsample_") }
	}
};

try {
	if (common.isNil) { Error("events.scd: generator/common.scd does not load").throw };
	#fail, server = common.("events.scd", \scsynth_dirt_events);
	// A NodeWatcher reads server.addr.asArray when it is made, and the cache of
	// the buffers listens to server.addr, so both come before the recorder.
	NodeWatcher.newFrom(server);
	Buffer.initServerCache(server);
	// Server:openBundle asks the address hasBundle, and BundleNetAddr.copyFrom
	// reads its addr, port, hostname and socket. Server:closeBundle ends with
	// sendClumpedBundles(time, *messages) on the address. A function of an
	// Event gets the Event as its first argument. nil as a value removes a key
	// of an Event, so a bundle is an array of its time and its messages.
	recorder = (
		addr: server.addr.addr,
		port: server.addr.port,
		hostname: server.addr.hostname,
		hasBundle: false,
		bundles: List.new,
		sendClumpedBundles: { |self, time ... messages| self[\bundles].add([time, messages]) },
		sendBundle: { |self, time ... messages| self[\bundles].add([time, messages]) },
		sendMsg: { |self ... item| self[\bundles].add([nil, [item]]) },
		sendRaw: { fail.("a raw packet reaches the address of the server") }
	);
	server.instVarPut(\addr, recorder);

	argument = { |item|
		case
		{ item.isNil } { "null" }
		{ item.isKindOf(Integer) or: { item.isKindOf(Float) } } { number.(item) }
		{ item.isKindOf(Symbol) or: { item.isKindOf(String) } } { string.(item) }
		{ item.isKindOf(List) } { "[" ++ item.collect(argument).join(", ") ++ "]" }
		{ fail.("cannot write the argument %".format(item.asCompileString)) }
	};

	run = { |name, channels, steps|
		var stride = if (channels == 1) { 4 } { 3 * channels };
		var gate = "dirt_gate" ++ channels;
		var dirt = dirtClass.newCopyArgs(channels, server);
		var environment = (dirt: dirt);
		var buffer = firstBuffer;
		var bundles = 0;
		var orbits, entries, start, sounds;
		server.newNodeAllocators;
		dirt.modules = [];
		dirt.soundLibrary = libraryClass.new(server, channels);
		["core-modules.scd", "core-synths-global.scd", "core-synths.scd"].do { |file|
			environment.use { (synths +/+ file).load };
		};
		if (dirt.modules.collect(_.name) != modules) {
			fail.("the core files add the modules %".format(dirt.modules.collect(_.name)));
		};
		dirt.initVowels(\counterTenor);
		dirt.instVarPut(\group, server.nextPermNodeID);
		dirt.instVarPut(\flotsam, IdentityDictionary.new);
		orbits = orbitCount.collect { |index|
			var orbit = orbitClass.newCopyArgs(dirt, outBus, index);
			var bus = { |offset|
				Bus.new(\audio, firstBus + (index * stride) + offset, channels, server)
			};
			orbit.instVarPut(\server, server);
			orbit.instVarPut(\group, server.nextPermNodeID);
			orbit.instVarPut(\synthBus, bus.(0));
			orbit.instVarPut(\dryBus, bus.(channels));
			orbit.instVarPut(\globalEffectBus, bus.(2 * channels));
			orbit.minSustain = 8 / sampleRate;
			orbit.initDefaultGlobalEffects;
			orbit.makeDefaultParentEvent;
			orbit
		};
		dirt.orbits = orbits;
		entries = library.collect { |entry|
			"      {\"name\": %, \"samples\": [%]}".format(string.(entry[0]),
				entry[1].collect { |sample, index|
					dirt.soundLibrary.addBuffer(entry[0],
						Buffer.new(server, sample[0], sample[1], buffer, sample[2]), index > 0);
					buffer = buffer + 1;
					"{\"buffer\": %, \"frames\": %, \"channels\": %, \"sample_rate\": %}"
						.format(buffer - 1, sample[0], sample[1], number.(sample[2]))
				}.join(", "))
		};
		if (take.().notEmpty) { fail.("the instance of % sends a bundle".format(name)) };
		start = orbits.collect { |orbit|
			var sent;
			orbit.initNodeTree;
			sent = take.();
			if (sent.size != 1) {
				fail.("initNodeTree of an orbit of % sends % bundles".format(name, sent.size));
			};
			bundles = bundles + 1;
			bundle.(sent[0], gate, "      ")
		};
		sounds = steps.collect { |step|
			var kind = step[0];
			case
			{ kind == \set } {
				orbits[step[1]].set(step[2], step[3]);
				if (take.().notEmpty) { fail.("the default % of % sends a bundle".format(step[2], name)) };
				"      {\"set\": {\"orbit\": %, \"name\": %, \"value\": %}}"
					.format(step[1], string.(step[2]), parameter.(step[3]))
			}
			{ kind == \play or: { kind == \drop } } {
				var when = step[1], pairs = step[2], sent;
				var event = (latency: when, timeStamp: when).putPairs(pairs);
				var names = pairs.select { |item, index| index.even };
				if (names.asSet.size != names.size) {
					fail.("% names a parameter twice".format(pairs));
				};
				try {
					eventClass.new(orbits @@ (event[\orbit] ? 0), dirt.modules, event).play;
				} { |error|
					fail.("SuperDirt throws on % at %: %".format(pairs, when, error.errorString));
				};
				sent = take.();
				if (kind == \drop) {
					if (sent.notEmpty) { fail.("SuperDirt plays % at %".format(pairs, when)) };
				} {
					if (sent.isEmpty) { fail.("SuperDirt drops % at %".format(pairs, when)) };
					if (sent.last[1].any(player).not) {
						fail.("% at % plays no sample".format(pairs, when));
					};
				};
				bundles = bundles + sent.size;
				"      {\"sound\": {\"when\": %, \"parameters\": {%}}, \"bundles\": %}".format(
					number.(when),
					pairs.clump(2).collect { |pair| string.(pair[0]) ++ ": " ++ parameter.(pair[1]) }
						.join(", "),
					list.(sent.collect { |each| bundle.(each, gate, "        ") }, "      "))
			}
			{ fail.("a step of kind %".format(kind)) }
		};
		"events.scd: scenario % (steps: %, bundles: %)".format(name, steps.size, bundles).postln;
		"  {\n"
		++ "    \"name\": %,\n".format(string.(name))
		++ ("    \"config\": {\"channels\": %, \"orbits\": %, \"first_bus\": %, \"out_bus\": %, "
			++ "\"sample_rate\": %},\n").format(channels, orbitCount, firstBus, outBus,
			number.(sampleRate))
		++ "    \"library\": %,\n".format(list.(entries, "    "))
		++ "    \"start\": %,\n".format(list.(start, "    "))
		++ "    \"steps\": %\n".format(list.(sounds, "    "))
		++ "  }"
	};

	synths = dirtClass.filenameSymbol.asString.dirname.dirname +/+ "synths";
	texts = scenarios.collect { |scenario| run.(*scenario) };
	text = "{\"scenarios\": " ++ list.(texts, "") ++ "}\n";
	File.use(fixtures +/+ "events.json", "w", { |file| file.write(text) });
	"events.scd: wrote events.json (scenarios: %, bytes: %)".format(texts.size, text.size).postln;
	0.exit;
} { |error|
	error.reportError;
	1.exit;
};
)
