// What both generators share. Each script loads this file and calls the
// function that it gives with its own name and the name of its server
// object:
//
//   #fail, server = common.("events.scd", \scsynth_dirt_events);
//
// load gives nil, with no error, for a file that is missing or that does not
// compile (measured), so each script fails on nil before the call.
//
// The function exits with 1 unless sclang is 3.14.1, the clones of SuperDirt
// and of Vowel are at their commits with no changes to tracked files, and
// every class and method comes from the include paths of the configuration.
// Then it sets Server.default to a server object of its own on UDP 57999,
// and boots nothing. It gives fail, which posts its message after the name
// of the script and exits with 1, and the server.
//
// A class name that sclang lacks stops the compile of a script, which then
// never exits, and makes load give nil for this file (both measured). So the
// generators name the classes of the extensions by symbol.
{ |script, serverName|
	var includes = LanguageConfig.includePaths.collect(_.withTrailingSlash);
	var dirtClass = \SuperDirt.asClass;
	var vowelClass = \Vowel.asClass;
	// sclang 3.14.1 runs the code after 1.exit (measured), so the throw stops it.
	var fail = { |message|
		(script ++ ": " ++ message).postln;
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
	var server;
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
	// Server.default is the scsynth on UDP 57110 unless set. SuperDirt takes it
	// where it gets no server, and Synth.newPaused takes it from
	// Integer:asTarget.
	server = Server(serverName, NetAddr("127.0.0.1", 57999));
	Server.default = server;
	[fail, server]
}
