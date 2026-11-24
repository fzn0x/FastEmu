package javax.microedition.lcdui;

public interface Choice {
    int EXCLUSIVE = 1;
    int MULTIPLE = 2;
    int IMPLICIT = 3;
    int POPUP = 4;

    int size();

    String getString(int elementNum);

    int append(String stringPart, Image imagePart);

    int getSelectedIndex();

    int getSelectedFlags(boolean[] selectedArray_return);

    void setSelectedFlags(boolean[] selectedArray);

    void setSelectedIndex(int elementNum, boolean selected);

    boolean isSelected(int elementNum);

    void delete(int elementNum);
}
