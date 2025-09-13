function ultimate_ttt_hw()
%% Ultimate TicTacToe mit NeoPixel + Pushbuttons 3x3 keypad + 1 x Reset
clc; clear;

%% Hardware setup
port = 'COM5';
board = 'Due';
a = arduino(port, board, 'Libraries', 'Adafruit/NeoPixel');

numLeds = 256;         % 16x16 matrix
dataPin = 'D52';       % Pin zu NeoPixel Matrix

np = addon(a, 'Adafruit/NeoPixel', dataPin, numLeds);

gridIdx = [];  % Container

% Button pins (3x3 keypad + reset)
buttonPins = {'D50','D48','D46', ...
              'D44','D42','D40', ...
              'D38','D36','D34'}; 
resetPin   = 'D32';

for i=1:numel(buttonPins)
    configurePin(a, buttonPins{i}, 'DigitalInput');
end
configurePin(a, resetPin, 'DigitalInput');

%% Spielzustand
data.grossesBrett  = repmat(' ',3,3); % 3x3 Mini-boards
data.bretter       = repmat(' ',9,9); % 9x9 Matrix
data.spieler       = 'X';
data.naechstesMini = [1 1];

% Spielebrett zentrieren
startRow = 4;
startCol = 4;

% NeoPixel Matrix leeren
setAllBlack();


disp('Ultimate Tic Tac Toe gestartet. Drücke Reset zum Neustart.');

while true
    % Reset
    if readDigitalPin(a, resetPin) == 1
        resetSpiel();
        pause(0.4);    % basic debounce
        continue;
    end

    % Poll keypad 1..9
    for k = 1:9
        if readDigitalPin(a, buttonPins{k}) == 1
            % Debug output: anzeigen welches Button gedrückt wurde
            fprintf('Button %d pressed (pin %s)\n', k, buttonPins{k});

            relPos = keypadMapping(k);   % [row,col] im gewählten mini-board
            fprintf(' -> maps to mini-board cell [row=%d, col=%d]\n', relPos(1), relPos(2));

            miniR = data.naechstesMini(1);
            miniC = data.naechstesMini(2);

            if all(data.naechstesMini==0)
                [miniR, miniC] = findeFreiesMini(data);
                if isempty(miniR), continue; end
            end

            % absolute position in 9x9
            R = (miniR-1)*3 + relPos(1);
            C = (miniC-1)*3 + relPos(2);
            fprintf(' -> absolute position in 9x9: [R=%d, C=%d]\n', R, C);

            macheZug(R,C);
            pause(0.28); % debounce
        end
    end
    pause(0.03); % small idle so CPU is not busy-looping
end


%% Hilffunktionen

    function pos = keypadMapping(k)
        % Push buttons in Keypad Konfiguration:
        % 7 8 9
        % 4 5 6
        % 1 2 3
        map = {[1,1],[1,2],[1,3], ...
       [2,1],[2,2],[2,3], ...
       [3,1],[3,2],[3,3]};

        pos = map{k};
    end

    function [miniR,miniC] = findeFreiesMini(data_)
        [r,c] = find(data_.grossesBrett==' ');
        if isempty(r), miniR=[]; miniC=[]; else miniR=r(1); miniC=c(1); end
    end

    function macheZug(R,C)
        miniR = ceil(R/3);
        miniC = ceil(C/3);

        if data.bretter(R,C) ~= ' ', return; end
        if any(data.naechstesMini) && ~isequal([miniR miniC], data.naechstesMini), return; end
        if data.grossesBrett(miniR,miniC) ~= ' ', return; end

        data.bretter(R,C) = data.spieler;

        % Farben Spieler
        cellColor = 'blue';
        if data.spieler == 'O', cellColor = 'green'; end

        ledIndex = boardToPixel(R,C);
        safeWriteColor(np, ledIndex, cellColor);

        % Spielzustand prüfen
        lokalesBrett = data.bretter( (3*miniR-2):(3*miniR), (3*miniC-2):(3*miniC) );
        if pruefeGewinner(lokalesBrett, data.spieler)
            data.grossesBrett(miniR,miniC) = data.spieler;
            siegerFarbe = cellColor;
            leds = miniBoardPixels(miniR, miniC);
            safeWriteColor(np, leds, siegerFarbe);
        elseif all(lokalesBrett(:)~=' ')
            data.grossesBrett(miniR,miniC) = '-'; % Gleichstand mini-board
        end

        % Prüfung globaler Gewinner
        if pruefeGewinner(data.grossesBrett, data.spieler)
            disp(['Spieler ', data.spieler, ' GEWINNT das Spiel!']);
            return;
        elseif all(data.grossesBrett(:)~=' ')
            disp('Gesamtspiel endet unentschieden!');
            return;
        end

        % Suche nächstes mini-board
        feldR = mod(R-1,3)+1;
        feldC = mod(C-1,3)+1;
        if data.grossesBrett(feldR,feldC) == ' '
            data.naechstesMini = [feldR feldC];
        else
            data.naechstesMini = [0 0];
        end

        % Spieler switchen
        if data.spieler == 'X', data.spieler = 'O'; else data.spieler = 'X'; end

        printBoard();
    end

    function leds = miniBoardPixels(miniR, miniC)
        rows = (miniR-1)*3+1 : miniR*3;
        cols = (miniC-1)*3+1 : miniC*3;
        leds = zeros(1,9);
        idx = 0;
        for rr = rows
            for cc = cols
                idx = idx + 1;
                leds(idx) = boardToPixel(rr, cc);
            end
        end
    end

    function pixelIdx = boardToPixel(row, col)
    % 9x9 Matrix in NeoPixel Indexierung umwandeln

    matRow = startRow + (row-1);   % 1..16

    % Brett horizontal spiegeln (mein Fehler in der Logik)
    matCol = startCol + (9 - col);

    % % boundary check
    % if matRow < 1 || matRow>16 || matCol<1 || matCol>16
    %     error('boardToPixel: computed matrix coords outside 16x16. Adjust startRow/startCol.');
    % end

    if mod(matRow,2)==1
        pixelIdx = (matRow-1)*16 + matCol;
    else
        pixelIdx = (matRow-1)*16 + (17 - matCol);
    end
end

    function ok = pruefeGewinner(brett, spieler)
        ok = false;
        for ii = 1:3
            if all(brett(ii,:) == spieler) || all(brett(:,ii) == spieler)
                ok = true; return;
            end
        end
        if all(diag(brett) == spieler) || all(diag(flipud(brett)) == spieler)
            ok = true;
        end
    end

    function resetSpiel()
    data.grossesBrett  = repmat(' ',3,3);
    data.bretter       = repmat(' ',9,9);
    data.spieler       = 'X';
    data.naechstesMini = [1 1];

    % Alle LEDs ausschalten
    safeWriteColor(np, 1:numLeds, 'black');

    disp('Neues Spiel gestartet! Spieler X beginnt.');
end

function setAllBlack()
    safeWriteColor(np, 1:numLeds, 'black');
end


    function safeWriteColor(npObj, indices, color)
        % indices: scalar or vector of pixel indices (1-based)
        % color: color name string or RGB triplet (0..255) or 0..1 triplet
        rgb = colorToRGB(color);    % 1x3 uint8

        % Try several common call signatures that different add-on versions use.
        % 1) writeColor(np, indices, Nx3 uint8)   (some versions allow vector)
        try
            writeColor(npObj, indices, uint8(rgb));
            return;
        catch
            % swallow and try next
        end

        % 2) writeColor(np, index, r,g,b) per-pixel
        try
            if isvector(indices) && numel(indices)>1
                for ii = indices
                    writeColor(npObj, ii, uint8(rgb(1)), uint8(rgb(2)), uint8(rgb(3)));
                end
            else
                writeColor(npObj, indices, uint8(rgb(1)), uint8(rgb(2)), uint8(rgb(3)));
            end
            return;
        catch
        end

        % 3) writeColor(np, index, [r g b]) scalar index
        try
            if numel(indices)==1
                writeColor(npObj, indices, uint8(rgb));
                return;
            end
        catch
        end

        % 4) In case the add-on expects color name strings for single index
        try
            if ischar(color) || isstring(color)
                if isvector(indices) && numel(indices)>1
                    for ii = indices
                        writeColor(npObj, ii, color);
                    end
                else
                    writeColor(npObj, indices, color);
                end
                return;
            end
        catch
        end

        % All attempts failed -> show helpful diagnostics
        error('safeWriteColor: could not call writeColor with tried signatures. Run "methods(np)" and paste the output here so I can adapt to your NeoPixel add-on API.');
    end

    function rgb = colorToRGB(col)
        % Returns 1x3 uint8 vector for named color or numeric input.
        if ischar(col) || isstring(col)
            switch lower(char(col))
                case 'black', rgb = [0 0 0];
                case 'white', rgb = [255 255 255];
                case 'red',   rgb = [255 0 0];
                case 'green', rgb = [0 255 0];
                case 'blue',  rgb = [0 0 255];
                case 'yellow',rgb = [255 255 0];
                otherwise
                    error('colorToRGB: unknown color name "%s". Use standard names or an RGB triplet.', char(col));
            end
        elseif isnumeric(col) && numel(col)==3
            rgb = double(col(:))';
            % scale if values are 0..1
            if max(rgb)<=1, rgb = round(rgb*255); end
            rgb = uint8(min(max(rgb,0),255));
        else
            error('colorToRGB: provide color-name string or 3-element numeric RGB.');
        end
    end

    function printBoard()
        disp('-------------------');
        for r = 1:9
            rowStr = '';
            for c = 1:9
                if data.bretter(r,c) == ' '
                    rowStr = [rowStr, '. ']; % empty cell as dot
                else
                    rowStr = [rowStr, data.bretter(r,c), ' '];
                end
                % vertical separator every 3 cells
                if mod(c,3)==0 && c<9
                    rowStr = [rowStr, '| '];
                end
            end
            disp(rowStr)
            % horizontal separator every 3 rows
            if mod(r,3)==0 && r<9
                disp('------+-------+------');
            end
        end
        if all(data.naechstesMini==0)
            disp('Next mini-board: any free');
        else
            fprintf('Next mini-board: (%d,%d)\n', data.naechstesMini(1), data.naechstesMini(2));
        end
        disp('-------------------');
    end


end
