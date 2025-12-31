# Fuzzbunny-rs

A Rust reimplementation of the original [fuzzbunny](https://github.com/mixpanel/fuzzbunny/) JS library.

This implementation provides a similar API as the original, with various tweaks to better suit the specific requirements of the parent Rustscape project.

## TODO:

 - Adjust scoring algorithm to better suit the requirements of `rustscape`
 - Improve scoring + filtering performance
 - Introduce no-alloc and no-std features
 - Adjust API to better suit Rust ergonomics