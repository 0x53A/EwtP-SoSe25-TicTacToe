% test_tic_tac_toe.m

% Define the current game state (0 = empty, 1 = X, 2 = O)
currentState = [0 0 0; 0 1 0; 2 0 0]; % Example state
playerTurn = 1; % X's turn
proposedMove = [2, 1]; % Proposed move to row 2, column 1

% Call the tic_tac_toe function
[updatedState, isLegal] = tic_tac_toe(currentState, playerTurn, proposedMove);

% Display the results
if isLegal
    disp('Move is legal.');
    disp('Updated Game State:');
    disp(updatedState);
else
    disp('Move is illegal.');
end