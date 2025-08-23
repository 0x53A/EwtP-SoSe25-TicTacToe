function [updatedState, isLegal] = tic_tac_toe(currentState, playerTurn, proposedMove)
    % TIC_TAC_TOE Updates the game state for a Tic-Tac-Toe game.
    %   currentState: 3x3 matrix (0 = empty, 1 = X, 2 = O)
    %   playerTurn: 1 for X, 2 for O
    %   proposedMove: [row, col] for the move (1-based indexing)
    %   updatedState: Updated game state if the move is legal
    %   isLegal: Boolean indicating if the move is legal

    arguments (Input)
        currentState (3, 3) double
        playerTurn {mustBeMember(playerTurn, [1, 2])}
        proposedMove (1, 2) double {mustBePositive, mustBeInteger}
    end

    arguments (Output)
        updatedState (3, 3) double
        isLegal logical
    end

    % Initialize output
    isLegal = false;
    updatedState = currentState;

    % Check if the proposed move is within bounds
    row = proposedMove(1);
    col = proposedMove(2);

    if row >= 1 && row <= 3 && col >= 1 && col <= 3
        % Check if the cell is empty
        if currentState(row, col) == 0
            % Update the game state
            updatedState(row, col) = playerTurn;
            isLegal = true;
        end
    end
end
