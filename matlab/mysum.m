function [outputArg1,outputArg2] = mysum(inputArg1,inputArg2)
%MYSUM Summary of this function goes here
%   Detailed explanation goes here
arguments (Input)
    inputArg1
    inputArg2
end

arguments (Output)
    outputArg1
    outputArg2
end

outputArg1 = inputArg1 * 2;
outputArg2 = inputArg2 + 1;
end