function tic_tac_toe_ui()
    % Create a figure for the game
    fig = uifigure('Name', 'Tic Tac Toe', 'Position', [100, 100, 300, 300]);

    % Initialize game state
    currentState = zeros(3); % 0 = empty, 1 = X, 2 = O
    playerTurn = 1; % X's turn

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
       if currentState(row, col) == 0 % Check if the cell is empty
        proposedMove = [row, col]; % Define the proposed move
        [updatedState, isLegal] = tic_tac_toe(currentState, playerTurn, proposedMove); % Call the existing function

        if isLegal
            currentState = updatedState; % Update the game state
            if playerTurn == 1
                src.Text = 'X'; % Display X or O based on player turn
            else
                src.Text = 'O'; % Display X or O based on player turn
            end
            playerTurn = 3 - playerTurn; % Switch player turn
            checkWinner(); % Check for a winner after the move
        end
      end
    end

    % Function to check for a winner
    function checkWinner()
        % Check rows, columns, and diagonals for a win
        for i = 1:3
            if all(currentState(i, :) == 1) || all(currentState(:, i) == 1)
                uialert(fig, 'X wins!', 'Game Over');
                resetGame();
                return;
            elseif all(currentState(i, :) == 2) || all(currentState(:, i) == 2)
                uialert(fig, 'O wins!', 'Game Over');
                resetGame();
                return;
            end
        end
        if all(diag(currentState) == 1) || all(diag(flipud(currentState)) == 1)
            uialert(fig, 'X wins!', 'Game Over');
            resetGame();
            return;
        elseif all(diag(currentState) == 2) || all(diag(flipud(currentState)) == 2)
            uialert(fig, 'O wins!', 'Game Over');
            resetGame();
            return;
        end
        if all(currentState(:) ~= 0)
            uialert(fig, 'It''s a draw!', 'Game Over');
            resetGame();
        end
    end

    % Function to reset the game
    function resetGame()
        currentState = zeros(3);
        for row = 1:3
            for col = 1:3
                btn(row, col).Text = '';
            end
        end
        playerTurn = 1; % Reset to X's turn
    end
end