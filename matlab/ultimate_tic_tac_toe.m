%% Ultimate Tic Tac Toe
function ultimate_tic_tac_toe()

clear, clc

    % Haupt-Fenster
    fig = uifigure('Name','Ultimatives Tic Tac Toe','Position',[300 100 600 700]);

    % Spielzustand
    data.grossesBrett = repmat(' ',3,3);   % Gewinner der Mini-Bretter
    data.bretter      = repmat(' ',9,9);   % gesamtes 9x9 Brett (3x3 Mini-Bretter)
    data.spieler      = 'X';               % aktueller Spieler
    data.naechstesMini = [1 1];            % X muss im Mini-Brett [1,1] starten
    data.buttons      = gobjects(9,9);     % Buttons
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
        'ButtonPushedFcn',@(btn, event) resetSpiel());

    % Erstelle 9x9 Gitter von Buttons mit sichtbarer Trennung
    buttonGroesse = 50;
    abstand = 5;        % normaler Abstand innerhalb der Mini-Bretter
    abstandGross = 15;  % extra Abstand zwischen den Mini-Brettern
    offsetX = 50;
    offsetY = 630;

    for R = 1:9
        for C = 1:9
            % Zusätzlicher Abstand nach jedem dritten Feld
            extraX = floor((C-1)/3) * (abstandGross - abstand);
            extraY = floor((R-1)/3) * (abstandGross - abstand);

            posX = offsetX + (C-1)*(buttonGroesse+abstand) + extraX;
            posY = offsetY - (R-1)*(buttonGroesse+abstand) - extraY;

            data.buttons(R,C) = uibutton(fig,...
                'Text',' ',...
                'Position',[posX, posY, buttonGroesse, buttonGroesse],...
                'FontSize',18,...
                'BackgroundColor',[1 1 1],... % weiß am Anfang
                'ButtonPushedFcn',@(btn, event) macheZug(R,C));
        end
    end

    % Zustand speichern
    fig.UserData = data;

    %% --- Verschachtelte Funktion für Spielzüge ---
    function macheZug(R,C)
        data = fig.UserData;

        % Bestimme welches Mini-Brett (3x3) zum Zug gehört
        miniR = ceil(R/3);
        miniC = ceil(C/3);

        % Prüfen ob der Zug erlaubt ist
        if data.bretter(R,C) ~= ' '
            return; % Feld ist schon belegt
        end
        if any(data.naechstesMini) && ~isequal([miniR miniC], data.naechstesMini)
            return; % nicht das geforderte Mini-Brett
        end
        if data.grossesBrett(miniR,miniC) ~= ' '
            return; % Mini-Brett bereits gewonnen
        end

        % Zug setzen
        data.bretter(R,C) = data.spieler;
        data.buttons(R,C).Text = data.spieler;

        % Farbe setzen je nach Spieler
        if data.spieler == 'X'
            data.buttons(R,C).BackgroundColor = [0 0 1]; % blau
        else
            data.buttons(R,C).BackgroundColor = [0 1 0]; % grün
        end

        % Prüfen ob Mini-Brett gewonnen
        lokalesBrett = data.bretter((3*miniR-2):(3*miniR),(3*miniC-2):(3*miniC));
        
        if pruefeGewinner(lokalesBrett, data.spieler)
    data.grossesBrett(miniR,miniC) = data.spieler;

    % Mini-Brett komplett in Spielerfarbe einfärben
    if data.spieler == 'X'
        siegerFarbe = [0 0 1]; % blau
    else
        siegerFarbe = [0 1 0]; % grün
    end

    for r = (3*miniR-2):(3*miniR)
        for c = (3*miniC-2):(3*miniC)
            data.buttons(r,c).BackgroundColor = siegerFarbe;
        end
    end

        elseif all(lokalesBrett(:) ~= ' ')
            data.grossesBrett(miniR,miniC) = '-'; % Unentschieden
        end

        % Prüfen ob das gesamte Spiel gewonnen ist
        if pruefeGewinner(data.grossesBrett, data.spieler)
            data.status.Text = ['Spieler ',data.spieler,' gewinnt das Spiel!'];
            deaktiviereAlle(data.buttons);
            fig.UserData = data;
            return;
        elseif all(data.grossesBrett(:) ~= ' ')
            data.status.Text = 'Das Spiel endet unentschieden!';
            deaktiviereAlle(data.buttons);
            fig.UserData = data;
            return;
        end

        % Nächstes Mini-Brett bestimmen
        feldR = mod(R-1,3)+1; % relative Position im Mini-Brett
        feldC = mod(C-1,3)+1;
        if data.grossesBrett(feldR,feldC) == ' '
            data.naechstesMini = [feldR feldC];
        else
            data.naechstesMini = [0 0]; % freie Wahl
        end

        % Spieler wechseln
        data.spieler = wechsleSpieler(data.spieler);
        if any(data.naechstesMini)
            data.status.Text = sprintf('Spieler %s ist dran (Spiel [%d,%d] spielen)',...
                data.spieler,data.naechstesMini(1),data.naechstesMini(2));
        else
            data.status.Text = sprintf('Spieler %s ist dran (freie Wahl)', data.spieler);
        end

        % Zustand speichern
        fig.UserData = data;
    end

    %% --- Reset Funktion ---
    function resetSpiel()
        data.grossesBrett = repmat(' ',3,3);
        data.bretter      = repmat(' ',9,9);
        data.spieler      = 'X';
        data.naechstesMini = [0 0];

        for r = 1:9
            for c = 1:9
                data.buttons(r,c).Text = ' ';
                data.buttons(r,c).BackgroundColor = [1 1 1];
                data.buttons(r,c).Enable = 'on';
            end
        end

        data.status.Text = 'Spieler X ist dran';
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
