function tic_tac_toe_ui()
    % Create a figure for the game
    fig = uifigure('Name', 'Tic Tac Toe', 'Position', [100, 100, 300, 300]);

    % Initialize game state
    currentState = zeros(1, 9, 'uint8'); % 0 = empty, 1 = X, 2 = O (now a 1x9 array)
    playerTurn = uint8(1); % X's turn

    % Create buttons for the Tic Tac Toe grid
    for row = 1:3
        for col = 1:3
            btn(row, col) = uibutton(fig, 'Text', '', ...
                'Position', [(col-1)*100, (3-row)*100, 100, 100], ...
                'ButtonPushedFcn', @(src, event) buttonCallback(src, row, col));
        end
    end

    % Callback function for button press
    function buttonCallback(src, row, col)
       % Convert 2D position to 1D index (row-first as specified in tic_tac_toe.m)
       index = (row-1)*3 + col;
       
       if currentState(index) == 0 % Check if the cell is empty
           % Create input structure
           input = struct('current_state', currentState, ...
                         'player_turn', playerTurn, ...
                         'proposed_move', uint8(index));
                     
           % Call the game function
           output = tic_tac_toe(input);
           
           if output.was_legal
               currentState = output.new_state; % Update the game state
               if playerTurn == 1
                   src.Text = 'X'; % Display X based on player turn
               else
                   src.Text = 'O'; % Display O based on player turn
               end
               
               % Check if there is a winner using the winner field
               if output.winner == 1
                   uialert(fig, 'X wins!', 'Game Over');
                   resetGame();
                   return;
               elseif output.winner == 2
                   uialert(fig, 'O wins!', 'Game Over');
                   resetGame();
                   return;
               else
                   % If no winner, continue with the next turn
                   playerTurn = output.next_player_turn; % Update player turn
                   % If board is full, it's a draw
                   if all(currentState ~= 0)
                       uialert(fig, 'It''s a draw!', 'Game Over');
                       resetGame();
                   end
               end
           end
       end
    end

    % Function to reset the game
    function resetGame()
        currentState = zeros(1, 9, 'uint8');
        for row = 1:3
            for col = 1:3
                btn(row, col).Text = '';
            end
        end
        playerTurn = uint8(1); % Reset to X's turn
    end
end