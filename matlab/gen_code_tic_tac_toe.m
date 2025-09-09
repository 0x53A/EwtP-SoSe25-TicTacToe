% Configure codegen
cfg = coder.config('lib');            % Generate a static library interface
cfg.TargetLang = 'C';                 % Force ANSI-C, not C++
cfg.GenCodeOnly = true;               % Do not try to compile with host toolchain
cfg.GenerateMakefile = false;         % Skip host makefiles
cfg.GenerateReport = false;           % Optional: HTML report
cfg.EnableDynamicMemoryAllocation = false;  % No malloc/free
cfg.EnableOpenMP = false;             % No multithreading
cfg.SupportNonFinite = false;         % Drop NaN/Inf support for smaller code
cfg.SaturateOnIntegerOverflow = false; % Use wrap-around instead of runtime checks
cfg.UseBuiltinFFTWLibrary = false;
cfg.PurelyIntegerCode = true;

% Create struct type definitions
input_struct_type = coder.typeof(struct('current_state', uint8(zeros(1, 9)), ...
                                         'player_turn', uint8(0), ...
                                         'proposed_move', uint8(0)));


% Generate C code
codegen -config cfg tic_tac_toe -args {input_struct_type}

disp('Done generating code')
