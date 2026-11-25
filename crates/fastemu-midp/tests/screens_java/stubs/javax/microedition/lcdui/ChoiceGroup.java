package javax.microedition.lcdui;

public class ChoiceGroup extends Item implements Choice {
    public ChoiceGroup(String label, int choiceType) {}

    public ChoiceGroup(String label, int choiceType, String[] stringElements, Image[] imageElements) {}

    public int size() {
        return 0;
    }

    public String getString(int elementNum) {
        return null;
    }

    public int append(String stringPart, Image imagePart) {
        return 0;
    }

    public int getSelectedIndex() {
        return 0;
    }

    public int getSelectedFlags(boolean[] selectedArray_return) {
        return 0;
    }

    public void setSelectedFlags(boolean[] selectedArray) {}

    public void setSelectedIndex(int elementNum, boolean selected) {}

    public boolean isSelected(int elementNum) {
        return false;
    }

    public void delete(int elementNum) {}

    public void deleteAll() {}
}
