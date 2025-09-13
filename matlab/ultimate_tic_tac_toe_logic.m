function output = ultimate_tic_tac_toe_logic(input)
% ULTIMATE_TIC_TAC_TOE_LOGIC Pure game logic for Ultimate Tic-Tac-Toe.
% Designed for code generation / C-export.
%
% Input (fields, types):
%  - current_grid_state: uint8 9x9 array (rows, cols) representing full board
%      (0 = empty, 1 = player 1 (X), 2 = player 2 (O)). Stored as uint8.
%  - current_grid_winners: uint8 3x3 array with winners of mini-grids (0=none,1,2,3=? draw->3 or 255?)
%      We'll use 0=none, 1=player1, 2=player2, 3=draw.
%  - player_turn: uint8 scalar (1 or 2)
%  - proposed_move_grid: uint8 scalar 1..9 selecting the mini-grid (1=row-major index)
%  - proposed_move_cell: uint8 scalar 1..9 selecting the cell inside the mini-grid (1=row-major index)
%
% Output (fields, types):
%  - was_legal: uint8 (1 legal, 0 illegal)
%  - new_grid_state: uint8 9x9 array
%  - new_grid_winners: uint8 3x3 array
%  - next_player_turn: uint8 (1 or 2)
%  - winner: uint8 (0 none, 1 or 2 overall winner)
%  - next_grid: uint8 (0 = free choice, 1..9 = required mini-grid)
%
% The board layout and indexing is row-major. Mini-grid numbering:
% 1 2 3
% 4 5 6
% 7 8 9

coder.inline('always');

% Input validation
assert(isa(input.current_grid_state, 'uint8'));
assert(isa(input.player_turn, 'uint8'));
assert(isa(input.proposed_move_grid, 'uint8'));
assert(isa(input.proposed_move_cell, 'uint8'));
assert(isa(input.current_grid_winners, 'uint8'));

% Initialize outputs
output = struct();
output.was_legal = uint8(0);
output.new_grid_state = zeros(9,9,'uint8');
output.new_grid_winners = zeros(3,3,'uint8');
output.next_player_turn = input.player_turn;
output.winner = uint8(0);
output.next_grid = uint8(0);

% Copy state
output.new_grid_state = input.current_grid_state;
output.new_grid_winners = input.current_grid_winners;

% Validate proposed grid and cell
if input.proposed_move_grid < 1 || input.proposed_move_grid > 9 || input.proposed_move_cell < 1 || input.proposed_move_cell > 9
    % illegal
    output.was_legal = uint8(0);
    output.next_player_turn = input.player_turn;
    return;
end

% Map proposed_move_grid (1..9) to top-left coordinates of mini-grid
% Use integer arithmetic only for code generation (int32)
mini_idx = int32(input.proposed_move_grid) - int32(1); % 0-based
mini_r = idivide(mini_idx, int32(3));   % 0..2 (int32)
mini_c = rem(mini_idx, int32(3));       % 0..2 (int32)

% Map proposed_move_cell (1..9) to r,c within mini-grid
cell_idx = int32(input.proposed_move_cell) - int32(1);
cell_r = idivide(cell_idx, int32(3));   % 0..2 (int32)
cell_c = rem(cell_idx, int32(3));       % 0..2 (int32)

% Compute absolute board indices (1-based) as int32 for indexing
abs_r = mini_r*int32(3) + cell_r + int32(1); % int32
abs_c = mini_c*int32(3) + cell_c + int32(1); % int32

% Check if mini-grid already decided
if output.new_grid_winners(mini_r+1, mini_c+1) ~= 0
    % can't play in a decided mini-grid
    output.was_legal = uint8(0);
    output.next_player_turn = input.player_turn;
    return;
end

% Check if the target cell is empty
if output.new_grid_state(abs_r, abs_c) ~= 0
    output.was_legal = uint8(0);
    output.next_player_turn = input.player_turn;
    % If the move was illegal because the cell is occupied, the player
    % must still play in the same mini-grid. Enforce that by setting
    % next_grid to the proposed mini-grid number.
    output.next_grid = input.proposed_move_grid;
    return;
end

% Make the move
output.new_grid_state(abs_r, abs_c) = input.player_turn;
output.was_legal = uint8(1);

% Determine if this mini-grid now has a winner or draw
startR = mini_r*int32(3) + int32(1);
endR   = mini_r*int32(3) + int32(3);
startC = mini_c*int32(3) + int32(1);
endC   = mini_c*int32(3) + int32(3);
mini_block = output.new_grid_state(startR:endR, startC:endC);
mini_winner = checkMiniWinner(mini_block);
if mini_winner == 3
    output.new_grid_winners(mini_r+1, mini_c+1) = uint8(3); % draw
elseif mini_winner == 1 || mini_winner == 2
    output.new_grid_winners(mini_r+1, mini_c+1) = uint8(mini_winner);
end

% Check if the overall game has a winner
overall = reshape(output.new_grid_winners,3,3);
ovw = checkMiniWinnerOverall(overall);
if ovw == 1 || ovw == 2
    output.winner = uint8(ovw);
end

% Determine next mini-grid to play in based on the cell position within mini-grid
next_mini_r = cell_r; % int32 0..2
next_mini_c = cell_c;
next_mini_num = uint8(next_mini_r*int32(3) + next_mini_c + int32(1));

% If that mini-grid is available (winner==0), enforce it, else free choice (0)
if output.new_grid_winners(next_mini_r+int32(1), next_mini_c+int32(1)) == 0
    output.next_grid = next_mini_num;
else
    output.next_grid = uint8(0);
end

% Switch player
if input.player_turn == 1
    output.next_player_turn = uint8(2);
else
    output.next_player_turn = uint8(1);
end

end

function w = checkMiniWinner(block)
% Returns 0=no winner,1=player1,2=player2,3=draw
w = uint8(0);
% Rows
for i=1:3
    if all(block(i,:) == 1)
        w = uint8(1); return;
    elseif all(block(i,:) == 2)
        w = uint8(2); return;
    end
end
% Cols
for i=1:3
    if all(block(:,i) == 1)
        w = uint8(1); return;
    elseif all(block(:,i) == 2)
        w = uint8(2); return;
    end
end
% Diags
if all(diag(block) == 1) || all(diag(flipud(block)) == 1)
    w = uint8(1); return;
elseif all(diag(block) == 2) || all(diag(flipud(block)) == 2)
    w = uint8(2); return;
end
% Draw? if all non-zero
if all(block(:) ~= 0)
    w = uint8(3); return;
end

end

function ov = checkMiniWinnerOverall(overall)
% overall is 3x3 with values 0,1,2,3(draw)
ov = uint8(0);
% Treat draw (3) as non-player for win detection
for i=1:3
    if all(overall(i,:) == 1)
        ov = uint8(1); return;
    elseif all(overall(i,:) == 2)
        ov = uint8(2); return;
    end
end
for i=1:3
    if all(overall(:,i) == 1)
        ov = uint8(1); return;
    elseif all(overall(:,i) == 2)
        ov = uint8(2); return;
    end
end
if all(diag(overall) == 1) || all(diag(flipud(overall)) == 1)
    ov = uint8(1); return;
elseif all(diag(overall) == 2) || all(diag(flipud(overall)) == 2)
    ov = uint8(2); return;
end
end
