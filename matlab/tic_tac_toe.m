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

    % Type validation for code generation
    coder.inline('always');
    assert(isa(input.current_state, 'uint8'));
    assert(isa(input.player_turn, 'uint8'));
    assert(isa(input.proposed_move, 'uint8'));
    
    % Initialize output structure
    output = struct('was_legal', uint8(0), ...
                   'new_state', zeros(1, 9, 'uint8'), ...
                   'next_player_turn', uint8(0));
    
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
    end
end