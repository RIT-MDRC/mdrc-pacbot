searchState.loadedDescShard("core_pb", 0, "<code>bincode::serde::decode_from_slice</code> with …\n<code>bincode::serde::encode_to_vec</code> with …\nSystems for motor speed calculations\nSee <code>RobotName</code>, a unique identifier for each known robot\nSee <code>ThreadedSocket</code>, a simple poll-based wrapper around a …\nthis message lets game server clients know that a game …\nThe default port where <code>server_pb</code> should expect to find the …\nThe default port where <code>gui_pb</code> should expect to connect to …\nGrid units per inch\nGrid units per meter\nInches per grid unit\nInches per meter\nThe maximum number of nodes in the target path sent from …\nMillimeters per grid unit\nMillimeters per inch\nThe size of the OLED display on the robot\nThe size of the OLED display on the robot\nThe default port where <code>server_pb</code> should expect to find the …\nSystems for motor speed calculations\nA drive system with any number of omniwheels that can …\nReturns the argument unchanged.\nGiven signed motor speeds, find the velocity and angular …\nGet the speeds that each motor should turn for the given …\nCalls <code>U::from(self)</code>.\nA drive system with any number of omniwheels that can …\nThe “debug” level.\nCorresponds to the <code>Debug</code> log level.\nThe “error” level.\nCorresponds to the <code>Error</code> log level.\nThe “info” level.\nCorresponds to the <code>Info</code> log level.\nAn enum representing the available verbosity levels of the …\nAn enum representing the available verbosity level filters …\nA trait encapsulating the operations required of a logger.\nMetadata about a log message.\nBuilder for <code>Metadata</code>.\nA level lower than all log levels.\nThe type returned by <code>from_str</code> when the string doesn’t …\nThe “payload” of a log message.\nBuilder for <code>Record</code>.\nMessages passed between the various tasks\nFunctionality that all tasks must support\nThe statically resolved maximum log level.\nThe type returned by <code>set_logger</code> if <code>set_logger</code> has already …\nThe “trace” level.\nCorresponds to the <code>Trace</code> log level.\nThe “warn” level.\nCorresponds to the <code>Warn</code> log level.\nThe message body.\nSet <code>args</code>.\nGet a value from a type implementing <code>std::fmt::Debug</code>.\nGet a value from a type implementing <code>std::fmt::Display</code>.\nReturns the string representation of the <code>Level</code>.\nReturns the string representation of the <code>LevelFilter</code>.\nInvoke the builder and return a <code>Record</code>\nReturns a <code>Metadata</code> object.\nReturns a new builder.\nReturns a new builder.\nLogs a message at the debug level.\nDetermines if a log message with the specified metadata …\nLogs a message at the error level.\nThe source file containing the message.\nSet <code>file</code>\nThe source file containing the message, if it is a <code>&#39;static</code> …\nSet <code>file</code> to a <code>&#39;static</code> string.\nFlushes any buffered records.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nLogs a message at the info level.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nIterate through all supported logging levels.\nIterate through all supported filtering levels.\nThe structured key-value pairs associated with the message.\nSet <code>key_values</code>\nStructured logging.\nThe verbosity level of the message.\nSet <code>Metadata::level</code>.\nThe verbosity level of the message.\nSetter for <code>level</code>.\nThe line containing the message.\nSet <code>line</code>\nLogs the <code>Record</code>.\nThe standard logging macro.\nDetermines if a message logged at the specified level in …\nReturns a reference to the logger.\nReturns the most verbose logging level.\nReturns the most verbose logging level filter.\nReturns the current maximum log level.\nMetadata about the log directive.\nSet <code>metadata</code>. Construct a <code>Metadata</code> object with …\nThe module path of the message.\nSet <code>module_path</code>\nThe module path of the message, if it is a <code>&#39;static</code> string.\nSet <code>module_path</code> to a <code>&#39;static</code> string\nConstruct new <code>RecordBuilder</code>.\nConstruct a new <code>MetadataBuilder</code>.\nReceive a message from other tasks; may be cancelled\nReceive a message from other tasks\nSend a message to the given task\nSend a message to the given task\nSets the global logger to a <code>Box&lt;Log&gt;</code>.\nSets the global logger to a <code>&amp;&#39;static Log</code>.\nA thread-unsafe version of <code>set_logger</code>.\nSets the global maximum log level.\nA thread-unsafe version of <code>set_max_level</code>.\nThe name of the target of the directive.\nSet <code>Metadata::target</code>\nThe name of the target of the directive.\nSetter for <code>target</code>.\nCreate a new <code>RecordBuilder</code> based on this record.\nConverts <code>self</code> to the equivalent <code>Level</code>.\nConverts the <code>Level</code> to the equivalent <code>LevelFilter</code>.\nLogs a message at the trace level.\nLogs a message at the warn level.\nAn error encountered while working with structured data.\nA key in a key-value.\nA source of key-values.\nA type that can be converted into a <code>Key</code>.\nA type that can be converted into a <code>Value</code>.\nA value in a key-value.\nA visitor for the key-value pairs in a <code>Source</code>.\nA visitor for a <code>Value</code>.\nA visitor for the key-value pairs in a <code>Source</code>.\nGet a borrowed string from this key.\nCreate an error from a standard error type.\nGet a value from a type implementing <code>std::fmt::Debug</code>.\nGet a value from a type implementing <code>std::fmt::Display</code>.\nCount the number of key-values that can be visited.\nTry downcast this value to <code>T</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet a value from a type implementing <code>ToValue</code>.\nGet a value from a type implementing <code>std::fmt::Debug</code>.\nGet a value from a type implementing <code>std::fmt::Display</code>.\nGet a value from a dynamic <code>std::fmt::Debug</code>.\nGet a value from a dynamic <code>std::fmt::Display</code>.\nGet a key from a borrowed string.\nGet the value for a given key.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCheck whether this value can be downcast to <code>T</code>.\nCreate an error from a message.\nGet a <code>null</code> value.\nSources for key-values.\nTry convert this value into a <code>bool</code>.\nTry convert this value into a borrowed string.\nTry convert this value into a <code>char</code>.\nTry convert this value into a <code>f64</code>.\nTry convert this value into a <code>i128</code>.\nTry convert this value into a <code>i64</code>.\nPerform the conversion.\nTry convert this value into a <code>u128</code>.\nTry convert this value into a <code>u64</code>.\nPerform the conversion.\nStructured values.\nVisit key-values.\nInspect this value using a simple visitor.\nVisit a <code>Value</code>.\nVisit a boolean.\nVisit a string.\nVisit a Unicode character.\nVisit a floating point.\nVisit a big signed integer.\nVisit a signed integer.\nVisit an empty value.\nVisit a key-value pair.\nVisit a key-value pair.\nVisit a string.\nVisit a big unsigned integer.\nVisit an unsigned integer.\nA source of key-values.\nA visitor for the key-value pairs in a <code>Source</code>.\nA visitor for the key-value pairs in a <code>Source</code>.\nCount the number of key-values that can be visited.\nGet the value for a given key.\nVisit key-values.\nVisit a key-value pair.\nVisit a key-value pair.\nAn error encountered while working with structured data.\nA type that can be converted into a <code>Value</code>.\nA value in a key-value.\nA visitor for a <code>Value</code>.\nA visitor for a <code>Value</code>.\nPerform the conversion.\nVisit a <code>Value</code>.\nVisit a <code>Value</code>.\nVisit a boolean.\nVisit a boolean.\nVisit a string.\nVisit a string.\nVisit a Unicode character.\nVisit a Unicode character.\nVisit a floating point.\nVisit a floating point.\nVisit a big signed integer.\nVisit a big signed integer.\nVisit a signed integer.\nVisit a signed integer.\nVisit an empty value.\nVisit an empty value.\nVisit a string.\nVisit a string.\nVisit a big unsigned integer.\nVisit a big unsigned integer.\nVisit an unsigned integer.\nVisit an unsigned integer.\nFunctionality that robots with motors must support\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nThe “main” method for the motors task\nSet PWM for the given pin\nFunctionality that robots with networking must support\nConnect to a network with the given username/password. …\nDisconnect from any active wifi network\nSee …\nReturns the argument unchanged.\nReturns the argument unchanged.\nSee …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nList information for up to <code>C</code> networks\nGet the device’s mac address\nSee …\nSee …\nThe “main” method for the network task\nRead (blocking) some bytes emitted by defmt\nReboot the microcontroller, as fully as possible\nAccept a socket that meets the requirements. Close the …\nDispose of the current socket\nIf the device is currently connected to a wifi network, …\nSee …\nFunctionality that robots with peripherals must support\nThe “main” method for the peripherals task\nWidth and height of a <code>Grid</code>.\nA 2D grid\nA <code>Grid</code> with precomputed data for faster pathfinding.\nA rectangle representing a wall.\nReturns the shortest path, if one exists, from start to …\nThe bottom right corner of the <code>Wall</code>.\nReturns the index of the given position in the …\nReturns the distance between two points, or <code>None</code> if the …\nnote that all walkable nodes might not be reachable from …\nFind the direction from the start point to the end point\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the underlying <code>Grid</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns all the walkable neighbors of the given position.\nReturn the walkable node from the nodes surrounding this …\nReturns the underlying <code>StandardGrid</code>, if one was used to …\nThe top left corner of the <code>Wall</code>.\nReturns the valid actions for the given position.\nwalkable, right, left, up, down\nValidates a <code>Grid</code>.\nReturns the positions of all walkable nodes in the grid.\nReturns whether there is a wall at a given position\nReturns the <code>Wall</code>s in the grid.\nwalls represent rectangles with top left corner at the …\nA (mostly) blank <code>Grid</code> - (1, 1) is walkable\nA <code>Grid</code> where the outermost path is empty\nThe official Pacbot <code>Grid</code>\nA <code>Grid</code> with many smaller paths to practice maneuvering\nGet the <code>ComputedGrid</code> associated with this enum\nReturns the argument unchanged.\nGet a list of all available grids\nGet the default Pacbot <code>Isometry2</code> associated with this enum\nGet the <code>Grid</code> associated with this enum\nGet the rectangles (in grid coordinates) that should be …\nGet the part of the <code>Grid</code> that should actually show on the …\nCalls <code>U::from(self)</code>.\nCancel an Over the Air Programming update for a robot\nClear Over the Air Programming update history for a robot\nContinue an Over the Air Programming update for a robot\nAfter a message is received\nAfter a connection is established, but before a message is …\nA connection could not be established\nSee <code>FrequentServerToRobot</code>\nThis is sent regularly and frequently to robots via …\nSend a message to the game server\nMessages sent from <code>gui_pb</code> to <code>server_pb</code>\n255 is reserved for raw bytes for logs\nSettings dictate that a connection should not be made\nRestart simulation (including rebuild)\nA button press (true) or release (false) for a simulated …\nSend a message to a robot\nThe display of a simulated robot\nThe positions of the simulated robots, to be shown in the …\nFirmware related items MUST remain first, or OTA …\nSet a robot’s target velocity (for WASD movement)\nSent from the robot peripherals task to the wifi task and …\nMessages sent from <code>server_pb</code> to <code>gui_pb</code>\nFirmware related items MUST remain first, or OTA …\nUpdate server settings\nLess frequent; includes updated server settings\nSend a message to the simulation\nMessages sent from <code>sim_pb</code> to <code>server_pb</code>\nInitiate an Over the Air Programming update for a robot\nVery frequent; includes all information about the status …\nSet a robot’s target location\nThe different async tasks that run on the robot\nThe absolute orientation of the robot, given by the IMU\nThe battery level of the robot\nThe grid cell the CV system thinks the robot is in\nReadings from the distance sensors, in order of angle 0, …\nWhether the robot should try to follow the target path …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe best guess location of the robot\nWhich pwm pin corresponds to which motor\nRequested velocity for each individual motor, forwards (+) …\nCreate one with default parameters of the given robot\nBasic parameters for the PID controller\nRequested output for each PWM pin, for testing\nThe points the robot should try to go to\nOverall requested velocity of the robot, ex. using WASD or …\nIndicates the last completed action\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nGeneric network connection settings\nGame server network options\nWASD, or right click to set target\nRarely changed options for the pacbot server\nAI\nPico network options, on-robot drive code options\nSimulation options\nNo movement\nTest (never goes back on itself)\nTest (random, uniform over all cells)\nWhen giving motor commands to the robot, should the …\nConfiguration; see <code>FrequentServerToRobot</code>\nWhether the app should try to connect/reconnect\nNetwork details\nNetwork details\nConnection settings\nWhether the robot should try to drive the target path\nOptions for pathing, speed\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nOptions for the go server\nHost a web server for browser clients\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nIP address, if it should be connected\nThe rotational speed, in rad/s, when driving with manual …\nThe translational speed, in gu/s, when driving with manual …\nWhich robot’s position should be used as the pacman …\nPort\nOptions for the robots\nWhich robots should be spawned in\nIn safe mode, only messages related to over the air …\nLaunch a fake game server and physics simulation as a …\nOptions for the simulation\nThe speed, in gu/s, to travel when the path length is 1, …\nThe maximum speed, in gu/s, when pathing autonomously\nThe speed, in gu/s, to add for each additional grid unit …\nWhich grid is current in use\nDetermines target position and path\nThe target speed of the robot in gu/s\nThe number of unique <code>RobotName</code>s\nRepresents a unique robot, either a physical device or a …\nThe default pre-filled ip - robots need not necessarily …\nReturns the argument unchanged.\nUniquely determine the robot name from the mac address, if …\nAll robot names in order\nCalls <code>U::from(self)</code>.\nWhether this robot is a raspberry pi pico\nWhether this robot is managed by the simulator\nThe mac address of this robot, must be unique\nThe port this robot will listen on for TCP connections\nThe characteristics of this robot\nAll the information that may vary from robot to robot\nDescribes physical characteristics of the motors\nWhich pwm pin corresponds to forwards and backwards for …\nDefault PID parameters - can change\nExposes methods to calculate motor velocities\nReturns the argument unchanged.\nReturns the argument unchanged.\nWhether the robot should expect to have access to a screen\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nDescribes physical characteristics of the motors\nCreate the default <code>RobotDefinition</code> for the given robot\nThe maximum value for motor PWM pins\nMaximum radius of the circle the robot fits into\nMaximum range of the sensors in meters\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nipv4 address with port number\nraw bytes\nthe type\nA TCP socket compatible with <code>ThreadedSocket</code>\ntext\nRepresents data that is either the given type, or text\nRepresents a type that is compatible with <code>ThreadedSocket</code>\nSimple poll-based wrapper around a socket (websocket or …\nRead new data from the socket (blocking await)\nQueue something to be sent to the socket (blocking await)\nSpecify an address to connect to (or None to suspend …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nClose the socket\nTry to connect to the address\nTry to read from the socket\nSend the data to the socket\nCreate a new <code>ThreadedSocket</code>\nRead new data from the socket, if it is available\nRuns on a separate thread to babysit the socket\nQueue something to be sent to the socket\nA future that yields the next message from the socket, or …\nFetch the latest information about the status of the …\nCreate a new ThreadedSocket with a name for logging\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.")