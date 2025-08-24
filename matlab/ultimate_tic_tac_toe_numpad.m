%% Tic Tac Toe mit Numpad

function ultimate_tic_tac_toe_numpad()

clear, clc

fig = uifigure('Name','Ultimatives Tic Tac Toe','Position',[300 100 600 700]);

% Spielzustand
data.grossesBrett = repmat(' ',3,3);   % Gewinner der Mini-Bretter
data.bretter      = repmat(' ',9,9);   % gesamtes 9x9 Brett (3x3 Mini-Bretter)
data.spieler      = 'X';               % aktueller Spieler
data.naechstesMini = [1 1];            % Start in Mini-Brett [1,1]
data.buttons      = gobjects(9,9);     % Buttons zum Anzeigen
data.status = uilabel(fig,'Text','Spieler X beginnt in Mini [1,1]',...
                        'Position',[150 20 300 30],...
                        'FontSize',16,...
                        'HorizontalAlignment','center');

% Reset-Button
uibutton(fig,...
    'Text','Reset',...
    'Position',[480 20 80 30],...
    'FontSize',14,...
    'BackgroundColor',[1 0 0],...
    'ButtonPushedFcn',@(btn,event) resetSpiel());

% Erstelle 9x9 Gitter von Buttons (nur zur Anzeige)
buttonGroesse = 50;
abstand = 5;
abstandGross = 15;
offsetX = 50;
offsetY = 630;

for R = 1:9
    for C = 1:9
        extraX = floor((C-1)/3) * (abstandGross - abstand);
        extraY = floor((R-1)/3) * (abstandGross - abstand);

        posX = offsetX + (C-1)*(buttonGroesse+abstand) + extraX;
        posY = offsetY - (R-1)*(buttonGroesse+abstand) - extraY;

        data.buttons(R,C) = uibutton(fig,...
            'Text',' ',...
            'Position',[posX, posY, buttonGroesse, buttonGroesse],...
            'FontSize',18,...
            'BackgroundColor',[1 1 1]);
    end
end

fig.UserData = data;

% --- Tastensteuerung ---
fig.WindowKeyPressFcn = @(src,event) tasteGedrueckt(event);

%% --- Verschachtelte Funktionen ---

function tasteGedrueckt(event)
    data = fig.UserData;

    % Mapping Numpad -> Position im Mini-Brett
    mapping = containers.Map(...
        {'numpad7','numpad8','numpad9','numpad4','numpad5','numpad6','numpad1','numpad2','numpad3'},...
        {[1,1],[1,2],[1,3],[2,1],[2,2],[2,3],[3,1],[3,2],[3,3]});

    taste = event.Key;
    if ~isKey(mapping,taste)
        return;
    end

    relPos = mapping(taste);
    miniR = data.naechstesMini(1);
    miniC = data.naechstesMini(2);

    % Wenn freie Wahl erlaubt
    if all(data.naechstesMini==0)
        [miniR,miniC] = findeFreiesMini(data);
        if isempty(miniR), return; end
    end

    % Absoluter Index im großen Brett
    R = (miniR-1)*3 + relPos(1);
    C = (miniC-1)*3 + relPos(2);

    macheZug(R,C);
end

function [miniR,miniC] = findeFreiesMini(data)
    [r,c] = find(data.grossesBrett==' ');
    if isempty(r)
        miniR=[]; miniC=[];
    else
        miniR=r(1); miniC=c(1);
    end
end

function macheZug(R,C)
    data = fig.UserData;

    miniR = ceil(R/3);
    miniC = ceil(C/3);

    if data.bretter(R,C) ~= ' '
        return;
    end
    if any(data.naechstesMini) && ~isequal([miniR miniC], data.naechstesMini)
        return;
    end
    if data.grossesBrett(miniR,miniC) ~= ' '
        return;
    end

    data.bretter(R,C) = data.spieler;
    data.buttons(R,C).Text = data.spieler;

    if data.spieler == 'X'
        data.buttons(R,C).BackgroundColor = [0 0 1];
    else
        data.buttons(R,C).BackgroundColor = [0 1 0];
    end

    lokalesBrett = data.bretter((3*miniR-2):(3*miniR),(3*miniC-2):(3*miniC));
    
    if pruefeGewinner(lokalesBrett, data.spieler)
        data.grossesBrett(miniR,miniC) = data.spieler;
        siegerFarbe = [0 0 1]*(data.spieler=='X') + [0 1 0]*(data.spieler=='O');
        for r=(3*miniR-2):(3*miniR)
            for c=(3*miniC-2):(3*miniC)
                data.buttons(r,c).BackgroundColor = siegerFarbe;
            end
        end
    elseif all(lokalesBrett(:)~=' ')
        data.grossesBrett(miniR,miniC) = '-';
    end

    if pruefeGewinner(data.grossesBrett, data.spieler)
        data.status.Text = ['Spieler ',data.spieler,' gewinnt das Spiel!'];
        deaktiviereAlle(data.buttons);
        fig.UserData = data;
        return;
    elseif all(data.grossesBrett(:)~=' ')
        data.status.Text = 'Das Spiel endet unentschieden!';
        deaktiviereAlle(data.buttons);
        fig.UserData = data;
        return;
    end

    feldR = mod(R-1,3)+1;
    feldC = mod(C-1,3)+1;
    if data.grossesBrett(feldR,feldC) == ' '
        data.naechstesMini = [feldR feldC];
    else
        data.naechstesMini = [0 0];
    end

    data.spieler = wechsleSpieler(data.spieler);
    if any(data.naechstesMini)
        data.status.Text = sprintf('Spieler %s ist dran (Spiel [%d,%d] spielen)',...
            data.spieler,data.naechstesMini(1),data.naechstesMini(2));
    else
        data.status.Text = sprintf('Spieler %s ist dran (freie Wahl)', data.spieler);
    end

    fig.UserData = data;
end

function resetSpiel()
    data.grossesBrett = repmat(' ',3,3);
    data.bretter      = repmat(' ',9,9);
    data.spieler      = 'X';
    data.naechstesMini = [1 1];

    for r = 1:9
        for c = 1:9
            data.buttons(r,c).Text = ' ';
            data.buttons(r,c).BackgroundColor = [1 1 1];
            data.buttons(r,c).Enable = 'on';
        end
    end

    data.status.Text = 'Spieler X beginnt in Mini [1,1]';
    fig.UserData = data;
end

end

%% --- Hilfsfunktionen ---
function ergebnis = pruefeGewinner(brett, spieler)
ergebnis = false;
for i = 1:3
    if all(brett(i,:) == spieler) || all(brett(:,i) == spieler)
        ergebnis = true; return;
    end
end
if all(diag(brett) == spieler) || all(diag(flipud(brett)) == spieler)
    ergebnis = true;
end
end

function deaktiviereAlle(buttons)
for r = 1:9
    for c = 1:9
        buttons(r,c).Enable = 'off';
    end
end
end

function naechster = wechsleSpieler(aktueller)
if aktueller == 'X'
    naechster = 'O';
else
    naechster = 'X';
end
end
