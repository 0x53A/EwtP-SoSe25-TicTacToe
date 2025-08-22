
     % Define the function inputs
   a = 0; % Example input
   b = 0; % Example input

   % Generate C code
   codegen mysum -args {a, b} -c
