Feature: Donut in the browser

  The classic 3D ASCII donut, rotating continuously. This is the default scene
  served by the aa-web shell.

  Scenario: Donut rotates and keeps rendering
    Given I open the scene page
    And I wait for the scene to render
    Then the terminal shows animated output
