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

% Create struct type definitions for Ultimate Tic-Tac-Toe
% - current_grid_state: 9x9 uint8 (rows, cols)
% - current_grid_winners: 3x3 uint8 with mini-grid winners (0=none,1,2,3=draw)
% - player_turn: uint8 scalar
% - proposed_move_grid: uint8 scalar 1..9 selecting mini-grid
% - proposed_move_cell: uint8 scalar 1..9 selecting cell inside mini-grid
input_struct_type = coder.typeof(struct(               ...
    'current_grid_state', uint8(zeros(9, 9)),            ...
    'current_grid_winners', uint8(zeros(3, 3)),          ...
    'player_turn', uint8(0),                             ...
    'proposed_move_grid', uint8(0),                      ...
    'proposed_move_cell', uint8(0)));


% Generate C code for the ultimate logic entry-point
codegen -config cfg ultimate_tic_tac_toe_logic -args {input_struct_type}

% Post-processing: Fix date/time and MATLAB Coder version in all generated files
disp('Fixing date/time stamps and version info in generated files (ultimate game)...');

% Get directory with generated code (ultimate_tic_tac_toe_logic)
codegenDir = fullfile(pwd, 'codegen', 'lib', 'ultimate_tic_tac_toe_logic');
if ~exist(codegenDir, 'dir')
    error('Generated code directory not found: %s', codegenDir);
end

% Find all C and H files
files = dir(fullfile(codegenDir, '**', '*.c'));
files = [files; dir(fullfile(codegenDir, '**', '*.h'))];

% Process each file
for i = 1:length(files)
    filePath = fullfile(files(i).folder, files(i).name);
    disp(['Processing: ' files(i).name]);
    
    % Process file line by line
    try
        % Read all lines
        fileID = fopen(filePath, 'r');
        if fileID == -1
            warning('Could not open file for reading: %s', filePath);
            continue;
        end
        
        lines = {};
        lineIdx = 1;
        while ~feof(fileID)
            lines{lineIdx} = fgetl(fileID);
            lineIdx = lineIdx + 1;
        end
        fclose(fileID);
        
        % Process lines - mark version and date lines for removal
        linesToRemove = false(1, length(lines));
        for j = 1:length(lines)
            if ~isempty(strfind(lines{j}, ' * MATLAB Coder version')) || ...
               ~isempty(strfind(lines{j}, ' * C/C++ source code generated on'))
                linesToRemove(j) = true;
            end
        end
        
        % Remove marked lines
        lines = lines(~linesToRemove);
        
        % Write back to file
        fileID = fopen(filePath, 'w');
        if fileID == -1
            warning('Could not open file for writing: %s', filePath);
            continue;
        end
        
        for j = 1:length(lines)
            fprintf(fileID, '%s\n', lines{j});
        end
        fclose(fileID);
        
        disp(['  Successfully processed: ' files(i).name]);
    catch ME
        warning('Error processing file %s: %s', filePath, ME.message);
    end
end

disp('Done generating code')
