Feature: DOOM in the browser

  The Vapor server streams text-mode DOOM to an xterm.js terminal over a
  WebSocket. This walkthrough boots it and drives it from the keyboard.

  Scenario: DOOM boots and responds to the keyboard
    Given I open the DOOM page
    When I wait for DOOM to start rendering
    And I press "Enter" to advance past the intro
    And I move with the arrow keys
    And I press "Space" to fire
    Then DOOM keeps rendering frames
