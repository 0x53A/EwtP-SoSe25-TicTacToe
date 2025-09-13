% Simple tests for ultimate_tic_tac_toe_engine
clear, clc

% Empty board
input.current_grid_state = zeros(9,9,'uint8');
input.current_grid_winners = zeros(3,3,'uint8');
input.player_turn = uint8(1);

% Play in mini-grid 1 (top-left), cell 5 (center of mini)
input.proposed_move_grid = uint8(1);
input.proposed_move_cell = uint8(5);

out = ultimate_tic_tac_toe_engine(input);
disp('After first move:');
disp(out.was_legal);
disp(out.next_grid);
disp(out.next_player_turn);
assert(out.was_legal == 1);
assert(out.next_grid == 5); % center cell maps to mini-grid 5
assert(out.next_player_turn == 2);

% Persist engine state for next call
input.current_grid_state = out.new_grid_state;
input.current_grid_winners = out.new_grid_winners;
input.player_turn = out.next_player_turn;

% Try illegal move: same cell again (same absolute spot -> same grid/cell)
input.proposed_move_grid = uint8(1);
input.proposed_move_cell = uint8(5);
out2 = ultimate_tic_tac_toe_engine(input);
disp('Illegal move attempt:');
disp(out2.was_legal);
assert(out2.was_legal == 0);

% Play in required mini-grid 5, choose cell 1
input.proposed_move_grid = uint8(5);
input.proposed_move_cell = uint8(1);
out3 = ultimate_tic_tac_toe_engine(input);
disp('Second legal move:');
disp(out3.was_legal);
disp(out3.next_grid);
assert(out3.was_legal == 1);

disp('All simple tests passed.');
