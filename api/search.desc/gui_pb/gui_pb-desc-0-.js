searchState.loadedDescShard("gui_pb", 0, "Stores all the data needed for the application\nDraw the main outer layout\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nLoad a replay from file\nA utility for recording over time\nSave the current replay to file\nTransforms between coordinate systems (such as …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nThe public interface for recording and replaying GUI data\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCreate a new ReplayManager; assumes that it is starting in …\nWhether playback is paused\nSpeed of playback - 0 is stopped, 1 is normal forwards\nWhen current_frame was played; used to determine when to …\nThe current replay, which may be recording or playing back\nReduce indentation\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nRed\nMain game grid\nKeybindings\nMotor configuration and testing\nGrey\nGreen\nStatus of OTA programming\nA generic status indication\nRobot view\nUser settings\nDetailed timings\nFor widgets that don’t have corresponding tabs\nYellow\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nPacbot’s real physical location, as determined by the […\nA collection of frames representing a full replay, along …\nThe metadata included in one frame of a <code>Replay</code>\nThe types of data that might be stored in a <code>ReplayFrame</code>\nGet the index of the current frame\nIndex of the most recently recorded or played frame\nThe data in the frame\nGet the number of frames\nThe data of the replay\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCreate a new Replay using bytes from a file\nReturns the most recent PacbotLocation\nGo back to the beginning of the recording\nGo to the end of the recording\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns whether the replay is at the beginning\nReturns whether the replay has played its last frame\nThe name/label given to this replay (usually matches the …\nIndex of the most recently recorded or played pacman …\nStart a new Replay\nIndex of the most recently recorded or played pacman state …\nAdd a pacman location to the end of the replay\nThe StandardGrid the recording uses\nThe time when recording started\nCreate a new Replay starting at the current frame in the …\nMoves to the previous frame, if it exists\nStep backwards until a PacmanState frame is reached\nMoves to the next frame, if it exists\nStep forwards until a PacmanState frame is reached\nGet the amount of time until the next frame\nGet the amount of time between the current and previous …\nWhen the data was created\nGet the bytes associated with the Replay\nA 2D transform consisting of per-axis scale and …\nIf true, x and y swap positions as part of the transform\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nReturns the inverse <code>Transform</code>. Panics if the …\nApplies a scalar transformation\nApplies the transformation to a point.\nApplies the transformation to a Point\nReturns the coordinates of the top left and bottom right …\nCreates a new <code>Transform</code> that maps the rect <code>(src_p1, src_p2)</code>…\nCreates a new <code>Transform</code> that maps the rect <code>(src_p1, src_p2)</code>…\nSwaps the X and Y components of this <code>Transform</code>.")