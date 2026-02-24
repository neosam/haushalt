## ADDED Requirements

### Requirement: Create Task Category
Household Owners and Admins (depending on hierarchy) SHALL be able to create task categories.

#### Scenario: Create category
- **WHEN** Owner/Admin creates category with name
- **THEN** category is created

#### Scenario: Optional properties
- **WHEN** category is created with color and sort order
- **THEN** these properties are stored

---

### Requirement: List Task Categories
Household members SHALL be able to see all categories in the household.

#### Scenario: List categories
- **WHEN** member requests categories
- **THEN** all categories are returned
- **THEN** name, color, and sort order are shown

---

### Requirement: View Category Details
Household members SHALL be able to view a specific category.

#### Scenario: View category
- **WHEN** member views category
- **THEN** name is shown
- **THEN** color is shown (if set)
- **THEN** sort order is shown

---

### Requirement: Update Category
Household Owners and Admins SHALL be able to modify categories.

#### Scenario: Update category
- **WHEN** Owner/Admin updates category
- **THEN** name, color, and sort order can be changed

---

### Requirement: Delete Category
Household Owners and Admins SHALL be able to remove categories.

#### Scenario: Delete category
- **WHEN** Owner/Admin deletes category
- **THEN** category is removed
- **THEN** tasks in that category have their category cleared
