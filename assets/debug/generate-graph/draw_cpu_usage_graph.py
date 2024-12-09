import json
import matplotlib.pyplot as plt
import os

# Directory where the JSON files are stored
data_directory = "tab_data"

# List to hold the cpu_usage values
cpu_usage_values = []

# Loop through the JSON files
for i in range(200):
    file_path = os.path.join(data_directory, f"data-{i}.json")
    
    # Check if the file exists
    if os.path.exists(file_path):
        with open(file_path, 'r') as file:
            data = json.load(file)
            # Extract the cpu_usage value from the first tab_info
            if 'tab_infos' in data and len(data['tab_infos']) > 0:
                cpu_usage = data['tab_infos'][0]['cpu_usage']
                cpu_usage_values.append(cpu_usage)
            else:
                cpu_usage_values.append(0)  # Append 0 if no data is found
    else:
        cpu_usage_values.append(0)  # Append 0 if the file does not exist

# Plotting the CPU usage values
plt.figure(figsize=(10, 5))
plt.plot(range(200), cpu_usage_values, marker='o', linestyle='-', color='r')
plt.title('CPU Usage from data-0.json to data-199.json')
plt.xlabel('File Index')
plt.ylabel('CPU Usage (%)')
plt.xticks(range(0, 200, 10))  # Set x-ticks for every 10 files
plt.grid()
plt.tight_layout()
plt.savefig('cpu_usage_graph.png')  # Save the graph as a PNG file
plt.show()  # Display the graph
