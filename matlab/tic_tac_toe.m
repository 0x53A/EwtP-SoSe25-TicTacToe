function output = tic_tac_toe(input)
    % TIC_TAC_TOE Updates the game state for a Tic-Tac-Toe game.
    %   input.current_state: 9-element array representing 3x3 board, row-first
    %                        (0 = empty, 1 = player 1, 2 = player 2)
    %   input.player_turn: 1 for player 1, 2 for player 2
    %   input.proposed_move: 1-based index into playing field (1-9)
    %
    %   output.was_legal: 1 if move was legal, 0 otherwise
    %   output.new_state: 9-element array of updated game state
    %   output.next_player_turn: next player's turn (1 or 2)
    %   output.winner: 0 for no winner, 1 or 2 for when player 1 or 2 have won

    % Type validation for code generation
    coder.inline('always');
    assert(isa(input.current_state, 'uint8'));
    assert(isa(input.player_turn, 'uint8'));
    assert(isa(input.proposed_move, 'uint8'));
    
    % Initialize output structure
    output = struct('was_legal', uint8(0), ...
                   'new_state', zeros(1, 9, 'uint8'), ...
                   'next_player_turn', uint8(0), ...
                   'winner', uint8(0));
    
    % Copy current state to new state
    output.new_state = input.current_state;
    
    % Determine if move is legal
    if input.proposed_move >= 1 && input.proposed_move <= 9
        % Check if the cell is empty (0)
        if input.current_state(input.proposed_move) == 0
            % Update the game state with player's mark
            output.new_state(input.proposed_move) = input.player_turn;
            output.was_legal = uint8(1);
            
            % Switch to the other player
            if input.player_turn == 1
                output.next_player_turn = uint8(2);
            else
                output.next_player_turn = uint8(1);
            end
        else
            % Cell already occupied - illegal move
            output.next_player_turn = input.player_turn; % Same player tries again
        end
    else
        % Move out of bounds - illegal move
        output.next_player_turn = input.player_turn; % Same player tries again
    end
    
    % If move was illegal, ensure was_legal is 0
    if output.was_legal == 0
        output.next_player_turn = input.player_turn;
    else
        % Check for winner, but only if the move was legal
        output.winner = checkWinner(output.new_state);
    end
end

function winner = checkWinner(state)
    % Check for winner in the current state
    % Returns 0 for no winner, 1 for player 1, 2 for player 2
    
    % Convert 1D array to 2D for easier win checking
    state2D = reshape(state, [3, 3]);
    winner = uint8(0);
    
    % Check rows
    for i = 1:3
        if all(state2D(i, :) == 1)
            winner = uint8(1);
            return;
        elseif all(state2D(i, :) == 2)
            winner = uint8(2);
            return;
        end
    end
    
    % Check columns
    for i = 1:3
        if all(state2D(:, i) == 1)
            winner = uint8(1);
            return;
        elseif all(state2D(:, i) == 2)
            winner = uint8(2);
            return;
        end
    end
    
    % Check diagonals
    if all(diag(state2D) == 1) || all(diag(flipud(state2D)) == 1)
        winner = uint8(1);
        return;
    elseif all(diag(state2D) == 2) || all(diag(flipud(state2D)) == 2)
        winner = uint8(2);
        return;
    end
    
    % No winner
    winner = uint8(0);
end