% test_tic_tac_toe.m

% Define the current game state (0 = empty, 1 = X, 2 = O)
currentState = uint8([0 0 0; 0 1 0; 2 0 0]); % Example state
playerTurn = uint8(1); % X's turn
proposedMove = uint8(4); % Converting 2D index [2,1] to 1D index (2-1)*3+1=4

% Flatten the 2D array to a 1D array
flatState = currentState(:)';

% Create input structure
input = struct('current_state', flatState, ...
              'player_turn', playerTurn, ...
              'proposed_move', proposedMove);

% Call the tic_tac_toe function
output = tic_tac_toe(input);

% Display the results
if output.was_legal
    disp('Move is legal.');
    disp('Updated Game State:');
    disp(reshape(output.new_state, [3, 3]));
    disp(['Next player turn: ', num2str(output.next_player_turn)]);
    
    if output.winner > 0
        disp(['Winner: Player ', num2str(output.winner)]);
    else
        disp('No winner yet.');
    end
else
    disp('Move is illegal.');
end