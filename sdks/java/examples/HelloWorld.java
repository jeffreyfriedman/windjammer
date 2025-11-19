import dev.windjammer.sdk.*;

/**
 * Hello World Example
 * 
 * The simplest possible Windjammer application in Java.
 * 
 * Build and run:
 *   mvn clean package
 *   java -cp target/windjammer-sdk-0.1.0.jar:examples HelloWorld
 */
public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("=== Windjammer Hello World (Java) ===");
        System.out.println("SDK Version: 0.1.0\n");
        
        // Create a new application
        var app = new App();
        
        // Add a simple system
        app.addSystem(() -> {
            System.out.println("Hello from the game loop!");
        });
        
        System.out.println("Application created successfully!");
        System.out.println("Systems registered: 1\n");
        System.out.println("Note: Full app.run() would start the game loop");
        System.out.println("For this example, we're just demonstrating SDK setup\n");
        
        // Run the application
        app.run();
    }
}

