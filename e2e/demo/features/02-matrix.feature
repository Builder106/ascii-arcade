Feature: Matrix rain in the browser

  Cascading green characters — the Matrix digital rain effect. Demonstrates
  switching scenes at runtime via the scene picker.

  Scenario: Matrix rain switches from the default scene
    Given I open the scene page
    When I select the "matrix" scene
    And I wait for the scene to render
    Then the terminal shows animated output
