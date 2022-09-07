# Wordle Optimizer

This is a tool to choose Wordle strategies.
See [this article](https://marksherry.dev/post/cheating-at-wordle/) for details as to how it works.

## Major features

* `single`: Best single word to play
* `pair`: Best two words to play, with the second word chosen to be most useful in the greatest number of cases
* `buckets`: Display possible outcomes and relative frequencies
* `best-second-hits`, `best-second-counts`, `best-second-precise`: Given an initial word, calculate which word would be best to follow, based on, respectively: total number of yellow or green letters; number of yellow letters and number of green letters; the precise response
* `solver`: Interactively ~cheat~ solve a Wordle puzzle by playing what it tells you to, and letting it know how well the guess did

