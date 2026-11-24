// Compile-time stub; the VM runs FastEmu's native class. See Connector.java.
package javax.microedition.io;

public interface HttpConnection extends ContentConnection {
    String HEAD = "HEAD";
    String GET = "GET";
    String POST = "POST";
    int HTTP_OK = 200;
    int HTTP_NO_CONTENT = 204;
    int HTTP_NOT_FOUND = 404;

    String getURL();
    String getProtocol();
    String getHost();
    String getFile();
    String getRef();
    String getQuery();
    int getPort();
    String getRequestMethod();
    void setRequestMethod(String method) throws java.io.IOException;
    String getRequestProperty(String key);
    void setRequestProperty(String key, String value) throws java.io.IOException;
    int getResponseCode() throws java.io.IOException;
    String getResponseMessage() throws java.io.IOException;
    long getExpiration() throws java.io.IOException;
    long getDate() throws java.io.IOException;
    long getLastModified() throws java.io.IOException;
    String getHeaderField(String name) throws java.io.IOException;
    int getHeaderFieldInt(String name, int def) throws java.io.IOException;
    long getHeaderFieldDate(String name, long def) throws java.io.IOException;
    String getHeaderField(int n) throws java.io.IOException;
    String getHeaderFieldKey(int n) throws java.io.IOException;
}
