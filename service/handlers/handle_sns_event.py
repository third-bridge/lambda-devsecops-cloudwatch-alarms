"""Post message to Slack from an CloudWatch alarm SNS notification.

See example message in [#artemis-alerts](https://grid-wearethirdbridge.enterprise.slack.com/archives/C08T0V9MKEG)
"""

import json
import os
import re
import urllib.request
from typing import Any

from aws_lambda_powertools.metrics import MetricUnit
from aws_lambda_powertools.utilities.typing import LambdaContext

from service.handlers.utils.observability import logger, metrics, tracer


@logger.inject_lambda_context()
@metrics.log_metrics
@tracer.capture_lambda_handler(capture_response=False)
def lambda_handler(event: dict[str, Any], context: LambdaContext):
    slack_webhook_url = os.environ.get('SLACK_WEBHOOK_URL', '')

    if not slack_webhook_url:
        logger.error('No SLACK_WEBHOOK_URL found in environment variables')
        exit(1)

    logger.debug(f'{event=}')
    logger.debug(f'{context=}')

    metrics.add_metric(name='sns_events_received', unit=MetricUnit.Count, value=1)

    for record in event['Records']:
        m = json.loads(record['Sns']['Message'])
        logger.debug(f'Received message: {m}')

        new_state = m['NewStateValue']
        color = '36a64f' if new_state == 'OK' else 'cb2431'

        region = m['AlarmArn'].split(':')[3]
        alarm_url = (
            f'https://{region}.console.aws.amazon.com/cloudwatch/home?region={region}#alarmsV2:alarm/{m["AlarmName"]}'
        )
        title = f'*{m["NewStateValue"]}: <{alarm_url}|{m["AlarmName"]}>*'

        reason = _quote_numbers_and_dates(m['NewStateReason'])

        slack_data = {
            'text': title,
            'attachments': [
                {
                    'fallback': title,
                    'footer': m['AlarmDescription'],
                    'color': color,
                    'fields': [
                        {'value': f'{reason}', 'short': False},
                    ],
                }
            ],
        }

        logger.info('Sending message to Slack')
        req = urllib.request.Request(
            slack_webhook_url, data=json.dumps(slack_data).encode('utf-8'), headers={'Content-Type': 'application/json'}
        )
        rsp = urllib.request.urlopen(req)
        logger.info(f'Slack response: {rsp.status} {rsp.reason}')
        if rsp.status != 200:
            logger.error(f'Error sending message to Slack: {rsp.status} {rsp.reason}')
            exit(1)

    metrics.add_metric(name='sns_events_responded', unit=MetricUnit.Count, value=1)


def _quote_numbers_and_dates(text: str) -> str:
    def replacer(match: re.Match[str]) -> str:
        # only keep 4 decimal places for floats
        if re.fullmatch(r'^\d+\.\d{4,}', match.group(0)):
            return f'`{float(match.group(0)):.4f}`'
        return f'`{match.group(0)}`'

    # Match numbers (including decimals) and date/time patterns
    pattern = r'\d+\.\d+|\d{2}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}|\d+'
    return re.sub(pattern, replacer, text)


_SAMPLE_SNS_MESSAGE = """
{
    'AlarmName': 'art1-expert-hub-cpu-low-utilization-tmp',
    'AlarmDescription': 'Alarm when ECS service CPU utilization is below 1% for 3 minutes',
    'AWSAccountId': '783764586577',
    'AlarmConfigurationUpdatedTimestamp': '2025-05-16T06:05:27.622+0000',
    'NewStateValue': 'ALARM',
    'NewStateReason': 'Threshold Crossed: 3 out of the last 3 datapoints [0.35220499833424884 (16/05/25 06:45:00), 0.2411688854917884 (16/05/25 06:44:00), 0.2608475057252993 (16/05/25 06:43:00)] were less than the threshold (1.0) (minimum 3 datapoints for OK -> ALARM transition).',
    'StateChangeTime': '2025-05-16T06:48:22.295+0000',
    'Region': 'EU (Ireland)',
    'AlarmArn': 'arn:aws:cloudwatch:eu-west-1:783764586577:alarm:art1-expert-hub-cpu-low-utilization-tmp',
    'OldStateValue': 'OK',
    'OKActions': [],
    'AlarmActions': ['arn:aws:sns:eu-west-1:783764586577:art1-expert-hub-cloudwatch-metric-alerts'],
    'InsufficientDataActions': [],
    'Trigger': {
        'MetricName': 'CPUUtilization',
        'Namespace': 'AWS/ECS',
        'StatisticType': 'Statistic',
        'Statistic': 'AVERAGE',
        'Unit': null,
        'Dimensions': [
            {'value': 'art1-expert-hub-service', 'name': 'ServiceName'},
            {'value': 'art1-cluster', 'name': 'ClusterName'},
        ],
        'Period': 60,
        'EvaluationPeriods': 3,
        'DatapointsToAlarm': 3,
        'ComparisonOperator': 'LessThanThreshold',
        'Threshold': 1.0,
        'TreatMissingData': 'missing',
        'EvaluateLowSampleCountPercentile': '',
    },
}
"""  # noqa: E501
