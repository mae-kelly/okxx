#!/bin/bash

# Bot monitoring script
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

while true; do
    clear
    echo -e "${GREEN}=== ARBITRAGE BOT MONITOR ===${NC}"
    echo "Time: $(date)"
    echo ""
    
    # Check if bot is running
    if pgrep -f "arb-scanner" > /dev/null; then
        echo -e "Status: ${GREEN}● RUNNING${NC}"
        
        # Show last 10 log lines
        echo -e "\nRecent Activity:"
        tail -n 10 logs/bot.log 2>/dev/null || echo "No logs yet"
        
        # Show system resources
        echo -e "\nSystem Resources:"
        ps aux | grep arb-scanner | grep -v grep | awk '{print "CPU: "$3"% | Memory: "$4"%"}'
    else
        echo -e "Status: ${RED}● STOPPED${NC}"
        echo "Run: ./start_bot.sh to start"
    fi
    
    sleep 5
done
