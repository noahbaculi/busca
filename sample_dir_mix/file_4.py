from bs4 import BeautifulSoup
from yahoo_fin.stock_info import *

# install requests
# install requests-html

# create symbols list with all securities to be considered
symbols_list = ["KO", "SPY", "VTI", "MUB", "MSEGX", "DBA", "DIS", "V", "JPM", "MSFT", "AAPL"]

df = pd.DataFrame(columns=["Symbol", "Security", "Type", "Category", "Price"])
pd.set_option("display.max_rows", 16)
pd.set_option("display.max_columns", 12)
pd.set_option("display.width", 200)

df["Symbol"] = symbols_list

# loop through all securities
for security_index in range(len(symbols_list)):
    # scrape Yahoo finance security profile page
    url_profile = "https://finance.yahoo.com/quote/%s/profile?p=%s" % (symbols_list[security_index], symbols_list[security_index])
    page_profile = requests.get(url_profile)
    soup_profile = BeautifulSoup(page_profile.content, "html.parser")
    # print(soup_profile.prettify())  # to examine imported profile HTML

    # see if security is a fund, otherwise assume company
    try:  # FUND
        fund_overview = soup_profile.find_all("div", {"class": "Mb(25px)"})[0].find(
            "h3").text.strip()  # look for fund overview section
        if fund_overview == "Fund Overview":
            security_name = soup_profile.find_all("div", {"class": "Mb(20px)"})[0].find("h3").text.strip()
            info_block = soup_profile.find_all("div", {
                "class": "Bdbw(1px) Bdbc($screenerBorderGray) Bdbs(s) H(25px) Pt(10px)"})
            try:  # see if security is a special type of fund
                if info_block[5].find("span", {"class": "Fl(start)"}).text.strip() == "Legal Type":
                    security_type = info_block[5].find("span", {"class": "Fl(end)"}).text.strip()
                else:
                    security_type = "Fund"
            except:
                raise SystemExit("An unknown error occured with the type of a fund security.")
            security_category = info_block[0].find("span", {"class": "Fl(end)"}).text.strip()
            df.at[security_index, "52-Week Change"] = "-"
            df.at[security_index, "Trailing P/E"] = "-"
            df.at[security_index, "Operating Margin"] = "-"
            df.at[security_index, "Quarterly Earnings Growth"] = "-"

            # scrape Yahoo finance security risk page
            url_risk = "https://finance.yahoo.com/quote/%s/risk?p=%s" % (symbols_list[security_index], symbols_list[security_index])
            page_risk = requests.get(url_risk)
            soup_risk = BeautifulSoup(page_risk.content, "html.parser")
            # print(soup_risk.prettify())  # to examine imported profile HTML

            # add the Beta (3Y Monthly) parameter
            if soup_risk.find_all("span", {"data-reactid": "44"})[0].text.strip() == "Beta":
                df.at[security_index, "Beta (3Y Monthly)"] = soup_risk.find_all("span", {"class": "W(39%) Fl(start)", "data-reactid": "46"})[0].text.strip()
            elif soup_risk.find_all("span", {"data-reactid": "74"})[0].text.strip() == "Beta":
                df.at[security_index, "Beta (3Y Monthly)"] = soup_risk.find_all("span", {"class": "W(39%) Fl(start)", "data-reactid": "76"})[0].text.strip()
            else:
                print("The Beta (3Y Monthly) parameter at %s is not in the expected location." % url_risk)

            # add the YTD Return parameter
            if soup_profile.find_all("span", {"data-reactid": "43"})[0].text.strip() == "YTD Return":
                df.at[security_index, "YTD Return"] = soup_profile.find_all("span", {"class": "Fl(end)", "data-reactid": "44"})[0].text.strip()
            elif soup_profile.find_all("span", {"data-reactid": "57"})[0].text.strip() == "YTD Return":
                df.at[security_index, "YTD Return"] = soup_profile.find_all("span", {"class": "Fl(end)", "data-reactid": "58"})[0].text.strip()
            else:
                print("The YTD Return parameter at %s is not in the expected location." % url_profile)

            # add the Annual Report Expense Ratio parameter
            try:
                if soup_profile.find_all("span", {"class": "Mend(5px) Whs(nw)", "data-reactid": "68"})[0].find("span").text.strip() == "Annual Report Expense Ratio (net)":
                    df.at[security_index, "Annual Expense Ratio"] = soup_profile.find_all("span", {"class": "W(20%) D(b) Fl(start) Ta(e)", "data-reactid": "70"})[0].text.strip()
            except:
                try:
                    if soup_profile.find_all("span", {"class": "Mend(5px) Whs(nw)", "data-reactid": "113"})[0].find("span").text.strip() == "Annual Report Expense Ratio (net)":
                        df.at[security_index, "Annual Expense Ratio"] = soup_profile.find_all("span", {"class": "W(20%) D(b) Fl(start) Ta(e)", "data-reactid": "115"})[0].text.strip()
                    else:
                        print("The Annual Report Expense Ratio parameter at %s is not in the expected location." % url_profile)
                except:
                    pass

            print("Success - Fund - %s" % symbols_list[security_index])
            print("Success - Fund Type - %s" % security_type)
            print("Success - Fund Cat - %s" % security_category)
        else:
            print("%s had a Yahoo Finance Profile page that could not be analyzed to find a fund overview section." %
                  symbols_list[security_index])

    except AttributeError:  # COMPANY
        security_name = soup_profile.find_all("div", {"class": "qsp-2col-profile Mt(10px) smartphone_Mt(20px) Lh(1.7)"})[0].find("h3").text.strip()
        security_type = "Company"
        security_category = soup_profile.find_all("span", {"class": "Fw(600)"})[0].text.strip()
        print("Success - Company - %s" % symbols_list[security_index])
        print("Success - Company Type - %s" % security_type)
        print("Success - Company Cat - %s" % security_category)

        # scrape Yahoo finance security statics page
        url_stats = "https://finance.yahoo.com/quote/%s/key-statistics?p=%s" % (symbols_list[security_index], symbols_list[security_index])
        page_stats = requests.get(url_stats)
        soup_stats = BeautifulSoup(page_stats.content, "html.parser")
        # print(soup_stats.prettify())  # to examine imported profile HTML

        # examine 2 locations for 52 Week Change parameter
        try:
            if soup_stats.find_all("span", {"data-reactid": "287"})[0].text.strip() == "52-Week Change":
                df.at[security_index, "52-Week Change"] = soup_stats.find_all("td", {"class": "Fz(s) Fw(500) Ta(end)"})[32].text.strip()
            else:
                print("The 52-Week Change parameter at %s is not in the expected location." % url_stats)
        except IndexError:
            try:
                if soup_stats.find_all("span", {"data-reactid": "292"})[0].text.strip() == "52-Week Change":
                    df.at[security_index, "52-Week Change"] = soup_stats.find_all("td", {"class": "Fz(s) Fw(500) Ta(end)"})[32].text.strip()
                else:
                    print("The 52-Week Change parameter at %s is not in the expected location." % url_stats)
            except IndexError:
                pass

        # add Trailing P/E parameter
        if soup_stats.find_all("span", {"data-reactid": "29"})[1].text.strip() == "Trailing P/E":
            df.at[security_index, "Trailing P/E"] = soup_stats.find_all("td", {"class": "Fz(s) Fw(500) Ta(end)"})[2].text.strip()

        # add Operating Margin parameter
        try:
            if soup_stats.find_all("span", {"data-reactid": "115"})[0].text.strip() == "Operating Margin":
                df.at[security_index, "Operating Margin"] = soup_stats.find_all("td", {"class": "Fz(s) Fw(500) Ta(end)", "data-reactid": "119"})[0].text.strip()
            else:
                print("The Operating Margin parameter at %s is not in the expected location." % url_stats)
        except IndexError:
            try:
                if soup_stats.find_all("span", {"data-reactid": "116"})[0].text.strip() == "Operating Margin":
                    df.at[security_index, "Operating Margin"] = soup_stats.find_all("td", {"class": "Fz(s) Fw(500) Ta(end)", "data-reactid": "120"})[0].text.strip()
                else:
                    print("The Operating Margin parameter at %s is not in the expected location." % url_stats)
            except IndexError:
                pass

        # add Quarterly Earnings Growth parameter
        try:
            if soup_stats.find_all("span", {"data-reactid": "195"})[0].text.strip() == "Quarterly Earnings Growth":
                df.at[security_index, "Quarterly Earnings Growth"] = soup_stats.find_all("td", {"class": "Fz(s) Fw(500) Ta(end)", "data-reactid": "199"})[0].text.strip()
            else:
                print("The Quarterly Earnings Growth parameter at %s is not in the expected location." % url_stats)
        except IndexError:
            try:
                if soup_stats.find_all("span", {"data-reactid": "197"})[0].text.strip() == "Quarterly Earnings Growth":
                    df.at[security_index, "Quarterly Earnings Growth"] = soup_stats.find_all("td", {"class": "Fz(s) Fw(500) Ta(end)", "data-reactid": "201"})[0].text.strip()
                else:
                    print("The Quarterly Earnings Growth parameter at %s is not in the expected location." % url_stats)
            except IndexError:
                pass

        # add Beta (3Y Monthly) parameter
        try:
            if soup_stats.find_all("span", {"data-reactid": "280"})[0].text.strip() == "Beta (3Y Monthly)":
                df.at[security_index, "Beta (3Y Monthly)"] = soup_stats.find_all("td", {"class": "Fz(s) Fw(500) Ta(end)", "data-reactid": "284"})[0].text.strip()
            elif soup_stats.find_all("span", {"data-reactid": "285"})[0].text.strip() == "Beta (3Y Monthly)":
                df.at[security_index, "Beta (3Y Monthly)"] = soup_stats.find_all("td", {"class": "Fz(s) Fw(500) Ta(end)", "data-reactid": "289"})[0].text.strip()
            else:
                print("The Beta (3Y Monthly) parameter at %s is not in the expected location." % url_stats)
        except IndexError:
            print("The Beta (3Y Monthly) parameter at %s is not in the expected location." % url_stats)

        df.at[security_index, "YTD Return"] = "-"
        df.at[security_index, "Annual Expense Ratio"] = "-"

    df.at[security_index, "Security"] = security_name  # assign name to dataframe
    df.at[security_index, "Type"] = security_type  # assign type to dataframe
    df.at[security_index, "Category"] = security_category  # assign category name to dataframe
    df.at[security_index, "Price"] = "%.2f" % (
        get_live_price(symbols_list[security_index]))  # get and assign price to dataframe
    print()

print(df)  # print dataframe