# Task: Fix Selection Broken Issue

## Issue Reference
- **Repository:** lassejlv/termy
- **Issue #:** 249
- **Title:** Selection broken
- **URL:** https://github.com/lassejlv/termy/issues/249
- **Status:** Open
- **Labels:** bug

## Problem Description
The text selection is broken when content is scrolling. When you scroll up and try to select something while content is still running and scrolling down, the selection doesn't keep the current buffer selected—it tries to follow the scrolling instead.

This behavior does not occur in ghostty.

## Expected Behavior
When selecting text and scrolling up while content is still running, the selection should remain fixed on the selected text/buffer, not try to follow the scrolling output.

## Actual Behavior
The selection tries to follow the scrolling content instead of maintaining the current buffer selection.

## Reproduction Steps
1. Start a command that produces continuous output
2. Scroll up while the output is still running
3. Try to select/mark some text
4. Observe that the selection follows the scrolling instead of staying fixed

## Priority
High - affects user experience when working with scrolling terminal output

## Notes
- Includes video demonstration in the original issue
- Works correctly in ghostty (reference implementation)
