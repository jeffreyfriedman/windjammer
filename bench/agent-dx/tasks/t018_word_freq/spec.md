The first line holds an integer K. The rest of the input is arbitrary ASCII text.

A word is a maximal run of ASCII letters and digits; everything else separates words. Words are compared case-insensitively (lower-case them). Print the K most frequent words, one per line as `word count`, ordered by count descending and ties broken by word ascending (byte order). If there are fewer than K distinct words print all of them.
